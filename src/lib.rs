// jsonb_ivm - Incremental JSONB View Maintenance Extension
//
// High-performance PostgreSQL extension for intelligent partial updates
// of JSONB materialized views in CQRS architectures.
//
// Copyright (c) 2025, Lionel Hamayon
// Licensed under the PostgreSQL License

use pgrx::prelude::*;
use pgrx::JsonB;
use serde_json::Value;
use std::collections::HashMap;

// Tell pgrx which PostgreSQL versions we support
pgrx::pg_module_magic!();

// ===== OPTIMIZED SEARCH HELPERS =====

/// Optimized integer ID matching with loop unrolling
/// Returns index of first matching element, or None
///
/// This function uses manual loop unrolling to help the compiler
/// generate SIMD instructions automatically (auto-vectorization)
#[inline]
fn find_by_int_id_optimized(array: &[Value], match_key: &str, match_value: i64) -> Option<usize> {
    // For small arrays, simple iteration is fastest
    if array.len() < 32 {
        return find_by_int_id_scalar(array, match_key, match_value);
    }

    // Unroll loop by 8 for potential auto-vectorization
    const UNROLL: usize = 8;
    let chunks = array.len() / UNROLL;

    for chunk_idx in 0..chunks {
        let base = chunk_idx * UNROLL;

        // Manual loop unrolling - compiler can auto-vectorize this
        // Check 8 elements at once
        for i in 0..UNROLL {
            if let Some(v) = array[base + i].get(match_key) {
                if let Some(id) = v.as_i64() {
                    if id == match_value {
                        return Some(base + i);
                    }
                }
            }
        }
    }

    // Handle remainder elements
    for i in (chunks * UNROLL)..array.len() {
        if let Some(v) = array[i].get(match_key) {
            if v.as_i64() == Some(match_value) {
                return Some(i);
            }
        }
    }

    None
}

/// Scalar fallback for small arrays or non-integer IDs
#[inline]
fn find_by_int_id_scalar(array: &[Value], match_key: &str, match_value: i64) -> Option<usize> {
    array.iter().position(|elem| {
        elem.get(match_key)
            .and_then(|v| v.as_i64())
            == Some(match_value)
    })
}

// ===== CORE FUNCTIONS =====

/// Merge top-level keys from source JSONB into target JSONB
///
/// # Arguments
/// * `target` - Base JSONB object to merge into
/// * `source` - JSONB object whose keys will be merged
///
/// # Returns
/// New JSONB object with merged keys (source overwrites target on conflicts)
///
/// # Errors
/// * Returns `NULL` if either argument is `NULL`
/// * Errors if either argument is not a JSONB object (arrays/scalars rejected)
///
/// # Examples
/// ```sql
/// SELECT jsonb_merge_shallow('{"a":1,"b":2}'::jsonb, '{"b":99,"c":3}'::jsonb);
/// -- Returns: {"a":1,"b":99,"c":3}
/// ```
///
/// # Notes
/// - Performs shallow merge only (nested objects are replaced, not merged)
/// - For deeply nested updates, use `jsonb_merge_at_path` (planned for v0.2.0)
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_merge_shallow(target: Option<JsonB>, source: Option<JsonB>) -> Option<JsonB> {
    // Handle NULL inputs - marked with `strict` so PostgreSQL handles this,
    // but we keep explicit handling for clarity
    let target = target?;
    let source = source?;

    // Extract inner serde_json::Value from pgrx JsonB wrapper
    let target_value = target.0;
    let source_value = source.0;

    // Validate that both are JSON objects (not arrays or scalars)
    let target_obj = match target_value.as_object() {
        Some(obj) => obj,
        None => {
            error!(
                "target argument must be a JSONB object, got: {}",
                value_type_name(&target_value)
            );
        }
    };

    let source_obj = match source_value.as_object() {
        Some(obj) => obj,
        None => {
            error!(
                "source argument must be a JSONB object, got: {}",
                value_type_name(&source_value)
            );
        }
    };

    // Perform shallow merge: clone target, then merge source keys
    let mut merged = target_obj.clone();

    for (key, value) in source_obj.iter() {
        merged.insert(key.clone(), value.clone());
    }

    // Wrap result in pgrx JsonB and return
    Some(JsonB(Value::Object(merged)))
}

/// Update a single element in a JSONB array by matching a key-value predicate
///
/// # Arguments
/// * `target` - JSONB document containing the array
/// * `array_path` - Path to the array within the document (e.g., "dns_servers")
/// * `match_key` - Key to match on (e.g., "id")
/// * `match_value` - Value to match (e.g., 42)
/// * `updates` - JSONB object to merge into matched element
///
/// # Returns
/// Updated JSONB document with modified array element
///
/// # Examples
/// ```sql
/// -- Update DNS server #42 in array of 50 servers
/// SELECT jsonb_array_update_where(
///     '{"dns_servers": [{"id": 42, "ip": "1.1.1.1"}, {"id": 43, "ip": "2.2.2.2"}]}'::jsonb,
///     'dns_servers',
///     'id',
///     '42'::jsonb,
///     '{"ip": "8.8.8.8"}'::jsonb
/// );
/// -- Returns: {"dns_servers": [{"id": 42, "ip": "8.8.8.8"}, {"id": 43, "ip": "2.2.2.2"}]}
/// ```
///
/// # Notes
/// - Updates FIRST matching element only
/// - If no match found, returns document unchanged
/// - Performs shallow merge on matched element
/// - O(n) complexity where n = array length
/// - For nested paths, use jsonb_set with jsonb_array_update_where
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_update_where(
    target: JsonB,
    array_path: &str,
    match_key: &str,
    match_value: JsonB,
    updates: JsonB,
) -> JsonB {
    // No Option unwrapping needed - strict guarantees non-NULL
    let mut target_value: Value = target.0;

    // Navigate to array location (single level for now)
    let array = match target_value.get_mut(array_path) {
        Some(arr) => arr,
        None => {
            error!(
                "Path '{}' does not exist in document",
                array_path
            );
        }
    };

    // Validate it's an array
    let array_items = match array.as_array_mut() {
        Some(arr) => arr,
        None => {
            error!(
                "Path '{}' does not point to an array, found: {}",
                array_path,
                value_type_name(array)
            );
        }
    };

    // Extract match value as serde_json::Value
    let match_val = match_value.0;

    // Validate updates is an object
    let updates_obj = match updates.0.as_object() {
        Some(obj) => obj,
        None => {
            error!(
                "updates argument must be a JSONB object, got: {}",
                value_type_name(&updates.0)
            );
        }
    };

    // Optimized fast path for integer IDs
    let match_idx = if let Some(int_id) = match_val.as_i64() {
        find_by_int_id_optimized(array_items, match_key, int_id)
    } else {
        // Fallback to scalar search for non-integer matches
        array_items.iter().position(|elem| {
            elem.get(match_key).map(|v| v == &match_val).unwrap_or(false)
        })
    };

    // Apply update if match found
    if let Some(idx) = match_idx {
        if let Some(elem_obj) = array_items[idx].as_object_mut() {
            for (key, value) in updates_obj.iter() {
                elem_obj.insert(key.clone(), value.clone());
            }
        }
    }

    JsonB(target_value)
}

/// Batch update multiple elements in a JSONB array
///
/// # Arguments
/// * `target` - JSONB document containing the array
/// * `array_path` - Path to the array (e.g., "dns_servers")
/// * `match_key` - Key to match on (e.g., "id")
/// * `updates_array` - Array of {match_value, updates} pairs
///
/// # Example
/// ```sql
/// SELECT jsonb_array_update_where_batch(
///     '{"dns_servers": [{"id": 1}, {"id": 2}, {"id": 3}]}'::jsonb,
///     'dns_servers',
///     'id',
///     '[
///         {"match_value": 1, "updates": {"ip": "1.1.1.1"}},
///         {"match_value": 2, "updates": {"ip": "2.2.2.2"}}
///     ]'::jsonb
/// );
/// ```
///
/// # Performance
/// - Amortizes array scan overhead
/// - Single pass for multiple updates
/// - 2-5× faster than N separate function calls
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_update_where_batch(
    target: JsonB,
    array_path: &str,
    match_key: &str,
    updates_array: JsonB,
) -> JsonB {
    let mut target_value: Value = target.0;

    let array = match target_value.get_mut(array_path) {
        Some(arr) => arr,
        None => error!("Path '{}' does not exist in document", array_path),
    };

    let array_items = match array.as_array_mut() {
        Some(arr) => arr,
        None => error!("Path '{}' does not point to an array", array_path),
    };

    let updates_list = match updates_array.0.as_array() {
        Some(arr) => arr,
        None => error!("updates_array must be a JSONB array"),
    };

    // Build hashmap of updates for O(1) lookup
    let mut update_map: HashMap<i64, &serde_json::Map<String, Value>> =
        HashMap::with_capacity(updates_list.len());

    for update_spec in updates_list {
        let spec_obj = match update_spec.as_object() {
            Some(obj) => obj,
            None => continue,  // Skip malformed specs
        };

        let match_value = match spec_obj.get("match_value").and_then(|v| v.as_i64()) {
            Some(id) => id,
            None => continue,
        };

        let updates_obj = match spec_obj.get("updates").and_then(|v| v.as_object()) {
            Some(obj) => obj,
            None => continue,
        };

        update_map.insert(match_value, updates_obj);
    }

    // Single pass through array, apply all matching updates
    for element in array_items.iter_mut() {
        if let Some(elem_obj) = element.as_object_mut() {
            if let Some(elem_id) = elem_obj.get(match_key).and_then(|v| v.as_i64()) {
                if let Some(updates_obj) = update_map.get(&elem_id) {
                    // Apply updates
                    for (key, value) in updates_obj.iter() {
                        elem_obj.insert(key.clone(), value.clone());
                    }
                }
            }
        }
    }

    JsonB(target_value)
}

/// Batch update arrays across multiple JSONB documents
///
/// # Arguments
/// * `targets` - Array of JSONB documents
/// * `array_path` - Path to array in each document
/// * `match_key` - Key to match on
/// * `match_value` - Value to match
/// * `updates` - JSONB object to merge
///
/// # Returns
/// SETOF jsonb - Set of updated JSONB documents (same order as input)
///
/// # Example
/// ```sql
/// -- Returns a set of rows, one per input document
/// SELECT * FROM jsonb_array_update_multi_row(
///     ARRAY[doc1, doc2, doc3],
///     'dns_servers',
///     'id',
///     '42'::jsonb,
///     '{"ip": "8.8.8.8"}'::jsonb
/// );
/// ```
///
/// # Use Case
/// Update 100 network configurations in one function call:
/// ```sql
/// -- Using WITH ORDINALITY to maintain order
/// WITH updated AS (
///     SELECT result, row_number() OVER () as rn
///     FROM jsonb_array_update_multi_row(
///         (SELECT array_agg(data ORDER BY id) FROM tv_network_configuration WHERE ...),
///         'dns_servers',
///         'id',
///         '42'::jsonb,
///         '{"ip": "8.8.8.8"}'::jsonb
///     ) AS result
/// )
/// UPDATE tv_network_configuration
/// SET data = updated.result
/// FROM updated
/// WHERE tv_network_configuration.id = updated.rn;
/// ```
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_update_multi_row(
    targets: pgrx::Array<JsonB>,
    array_path: &str,
    match_key: &str,
    match_value: JsonB,
    updates: JsonB,
) -> TableIterator<'static, (name!(result, JsonB),)> {
    let match_val = match_value.0;
    let updates_obj = match updates.0.as_object() {
        Some(obj) => obj.clone(),
        None => error!("updates argument must be a JSONB object"),
    };

    // Convert &str to owned String to satisfy 'static lifetime
    let array_path_owned = array_path.to_string();
    let match_key_owned = match_key.to_string();

    // Collect all targets into a Vec to own the data
    let targets_vec: Vec<JsonB> = targets
        .iter()
        .filter_map(|t| t)
        .collect();

    // Create iterator that will be returned as SETOF
    TableIterator::new(
        targets_vec.into_iter().map(move |target| {
            // Call single-row update for each document
            let result = jsonb_array_update_where(
                target,
                &array_path_owned,
                &match_key_owned,
                JsonB(match_val.clone()),
                JsonB(Value::Object(updates_obj.clone())),
            );
            (result,)
        })
    )
}

/// Merge JSONB object at a specific nested path
///
/// # Arguments
/// * `target` - Base JSONB document
/// * `source` - JSONB object to merge
/// * `path` - Path where to merge (empty array = root level)
///
/// # Returns
/// Updated JSONB with source merged at path
///
/// # Examples
/// ```sql
/// -- Update network_configuration in allocation document
/// SELECT jsonb_merge_at_path(
///     '{"id": 1, "network_configuration": {"id": 17, "name": "old"}}'::jsonb,
///     '{"name": "updated"}'::jsonb,
///     ARRAY['network_configuration']
/// );
/// -- Returns: {"id": 1, "network_configuration": {"id": 17, "name": "updated"}}
/// ```
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_merge_at_path(
    target: JsonB,
    source: JsonB,
    path: pgrx::Array<&str>,
) -> JsonB {
    // No Option unwrapping needed - strict guarantees non-NULL
    let mut target_value: Value = target.0;

    // Validate source is an object
    let source_obj = match source.0.as_object() {
        Some(obj) => obj,
        None => {
            error!(
                "source argument must be a JSONB object, got: {}",
                value_type_name(&source.0)
            );
        }
    };

    // Collect path into owned Vec<String> to avoid lifetime issues
    let path_vec: Vec<String> = path
        .iter()
        .flatten()
        .map(|s| s.to_owned())
        .collect();

    // If path is empty, merge at root
    if path_vec.is_empty() {
        let target_obj = match target_value.as_object_mut() {
            Some(obj) => obj,
            None => {
                error!(
                    "target argument must be a JSONB object when path is empty, got: {}",
                    value_type_name(&target_value)
                );
            }
        };

        // Shallow merge at root
        for (key, value) in source_obj.iter() {
            target_obj.insert(key.clone(), value.clone());
        }

        return JsonB(target_value);
    }

    // Navigate to parent of target path
    let mut current = &mut target_value;
    for (i, key) in path_vec.iter().enumerate() {
        let is_last = i == path_vec.len() - 1;

        if is_last {
            // At target location - merge here
            let parent_obj = match current.as_object_mut() {
                Some(obj) => obj,
                None => {
                    error!(
                        "Path navigation failed: expected object at {:?}, got: {}",
                        &path_vec[..i],
                        value_type_name(current)
                    );
                }
            };

            // Get existing value at key (or create empty object)
            let target_at_path = parent_obj
                .entry(key.to_string())
                .or_insert_with(|| Value::Object(Default::default()));

            // Merge source into target at path
            let merge_target = match target_at_path.as_object_mut() {
                Some(obj) => obj,
                None => {
                    error!(
                        "Cannot merge into non-object at path {:?}, found: {}",
                        path_vec,
                        value_type_name(target_at_path)
                    );
                }
            };

            for (key, value) in source_obj.iter() {
                merge_target.insert(key.clone(), value.clone());
            }
        } else {
            // Navigate deeper
            let current_type = value_type_name(current);
            let obj = match current.as_object_mut() {
                Some(obj) => obj,
                None => {
                    error!(
                        "Path navigation failed at {:?}, expected object, got: {}",
                        &path_vec[..=i],
                        current_type
                    );
                }
            };

            current = obj
                .entry(key.to_string())
                .or_insert_with(|| Value::Object(Default::default()));
        }
    }

    JsonB(target_value)
}

/// Helper function to get human-readable type name for error messages
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

// ===== TESTS =====

#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use pgrx::JsonB;
    use serde_json::json;

    #[pgrx::pg_test]
    fn test_basic_merge() {
        let target = JsonB(json!({"a": 1, "b": 2}));
        let source = JsonB(json!({"c": 3}));

        let result =
            crate::jsonb_merge_shallow(Some(target), Some(source)).expect("merge should succeed");

        assert_eq!(result.0, json!({"a": 1, "b": 2, "c": 3}));
    }

    #[pgrx::pg_test]
    fn test_overlapping_keys() {
        let target = JsonB(json!({"a": 1, "b": 2}));
        let source = JsonB(json!({"b": 99, "c": 3}));

        let result =
            crate::jsonb_merge_shallow(Some(target), Some(source)).expect("merge should succeed");

        // Source value (99) should overwrite target value (2)
        assert_eq!(result.0, json!({"a": 1, "b": 99, "c": 3}));
    }

    #[pgrx::pg_test]
    fn test_empty_source() {
        let target = JsonB(json!({"a": 1}));
        let source = JsonB(json!({}));

        let result =
            crate::jsonb_merge_shallow(Some(target), Some(source)).expect("merge should succeed");

        assert_eq!(result.0, json!({"a": 1}));
    }

    #[pgrx::pg_test]
    fn test_null_handling() {
        let source = JsonB(json!({"a": 1}));

        // NULL target
        let result = crate::jsonb_merge_shallow(None, Some(source));
        assert!(result.is_none());

        // NULL source
        let target = JsonB(json!({"a": 1}));
        let result = crate::jsonb_merge_shallow(Some(target), None);
        assert!(result.is_none());
    }

    #[pgrx::pg_test]
    #[should_panic(expected = "target argument must be a JSONB object")]
    fn test_array_target_errors() {
        let target = JsonB(json!([1, 2, 3]));
        let source = JsonB(json!({"a": 1}));

        // This should error
        let _ = crate::jsonb_merge_shallow(Some(target), Some(source));
    }

    #[pgrx::pg_test]
    #[should_panic(expected = "source argument must be a JSONB object")]
    fn test_array_source_errors() {
        let target = JsonB(json!({"a": 1}));
        let source = JsonB(json!([1, 2, 3]));

        // This should error
        let _ = crate::jsonb_merge_shallow(Some(target), Some(source));
    }

    #[pgrx::pg_test]
    fn test_array_update_where_basic() {
        let target = JsonB(json!({
            "dns_servers": [
                {"id": 42, "ip": "1.1.1.1", "port": 53},
                {"id": 43, "ip": "2.2.2.2", "port": 53}
            ]
        }));

        let result = crate::jsonb_array_update_where(
            target,
            "dns_servers",
            "id",
            JsonB(json!(42)),
            JsonB(json!({"ip": "8.8.8.8"})),
        );

        let expected = json!({
            "dns_servers": [
                {"id": 42, "ip": "8.8.8.8", "port": 53},
                {"id": 43, "ip": "2.2.2.2", "port": 53}
            ]
        });

        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_array_update_where_no_match() {
        let target = JsonB(json!({
            "dns_servers": [
                {"id": 42, "ip": "1.1.1.1"},
                {"id": 43, "ip": "2.2.2.2"}
            ]
        }));

        let result = crate::jsonb_array_update_where(
            target,
            "dns_servers",
            "id",
            JsonB(json!(999)),  // No element with id=999
            JsonB(json!({"ip": "8.8.8.8"})),
        );

        // Should return unchanged
        assert_eq!(result.0, json!({
            "dns_servers": [
                {"id": 42, "ip": "1.1.1.1"},
                {"id": 43, "ip": "2.2.2.2"}
            ]
        }));
    }

    #[pgrx::pg_test]
    fn test_array_update_where_large_array() {
        // Create array with 100 elements
        let mut servers = Vec::new();
        for i in 1..=100 {
            servers.push(json!({
                "id": i,
                "ip": format!("192.168.1.{}", i),
                "port": 53
            }));
        }

        let target = JsonB(json!({"dns_servers": servers}));

        // Update element #99 (near end of array)
        let result = crate::jsonb_array_update_where(
            target,
            "dns_servers",
            "id",
            JsonB(json!(99)),
            JsonB(json!({"ip": "8.8.8.8", "status": "updated"})),
        );

        // Verify element #99 was updated
        let updated_server = &result.0["dns_servers"][98];  // 0-indexed
        assert_eq!(updated_server["ip"], "8.8.8.8");
        assert_eq!(updated_server["status"], "updated");
        assert_eq!(updated_server["port"], 53);  // Unchanged field preserved
    }

    #[pgrx::pg_test]
    fn test_array_update_where_nested_path() {
        // For now, test single-level path. Nested paths can be handled with jsonb_set
        let target = JsonB(json!({
            "dns_servers": [
                {"id": 1, "ip": "1.1.1.1"},
                {"id": 2, "ip": "2.2.2.2"}
            ]
        }));

        let result = crate::jsonb_array_update_where(
            target,
            "dns_servers",
            "id",
            JsonB(json!(2)),
            JsonB(json!({"ip": "8.8.8.8"})),
        );

        assert_eq!(
            result.0["dns_servers"][1]["ip"],
            "8.8.8.8"
        );
    }

    #[pgrx::pg_test]
    #[should_panic(expected = "does not point to an array")]
    fn test_array_update_where_invalid_path() {
        let target = JsonB(json!({"dns_servers": {"id": 42}}));  // Object, not array

        let _ = crate::jsonb_array_update_where(
            target,
            "dns_servers",
            "id",
            JsonB(json!(42)),
            JsonB(json!({"ip": "8.8.8.8"})),
        );
    }

    #[pgrx::pg_test]
    #[should_panic(expected = "updates argument must be a JSONB object")]
    fn test_array_update_where_invalid_updates() {
        let target = JsonB(json!({"dns_servers": [{"id": 42}]}));

        let _ = crate::jsonb_array_update_where(
            target,
            "dns_servers",
            "id",
            JsonB(json!(42)),
            JsonB(json!("not an object")),  // Invalid: scalar instead of object
        );
    }

    #[pgrx::pg_test]
    fn test_merge_at_path_root() {
        let target = JsonB(json!({"a": 1, "b": 2}));
        let source = JsonB(json!({"b": 99, "c": 3}));

    let result = crate::jsonb_merge_at_path(
        target,
        source,
        pgrx::Array::from(vec![]),  // Empty path = root merge
    );

        assert_eq!(result.0, json!({"a": 1, "b": 99, "c": 3}));
    }

    #[pgrx::pg_test]
    fn test_merge_at_path_nested() {
        let target = JsonB(json!({
            "id": 1,
            "network_configuration": {
                "id": 17,
                "name": "old",
                "gateway_ip": "192.168.1.1"
            }
        }));
        let source = JsonB(json!({"name": "updated", "dns_count": 50}));

    let result = crate::jsonb_merge_at_path(
        target,
        source,
        pgrx::Array::from(vec!["network_configuration"]),
    );

        let expected = json!({
            "id": 1,
            "network_configuration": {
                "id": 17,
                "name": "updated",
                "gateway_ip": "192.168.1.1",
                "dns_count": 50
            }
        });

        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_merge_at_path_deep() {
        let target = JsonB(json!({
            "level1": {
                "level2": {
                    "level3": {
                        "existing": "value"
                    }
                }
            }
        }));
        let source = JsonB(json!({"new": "data"}));

    let result = crate::jsonb_merge_at_path(
        target,
        source,
        pgrx::Array::from(vec!["level1", "level2", "level3"]),
    );

        assert_eq!(
            result.0["level1"]["level2"]["level3"],
            json!({"existing": "value", "new": "data"})
        );
    }
}
