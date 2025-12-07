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

// Tell pgrx which PostgreSQL versions we support
pgrx::pg_module_magic!();

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
fn jsonb_merge_shallow(
    target: Option<JsonB>,
    source: Option<JsonB>,
) -> Option<JsonB> {
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

        let result = crate::jsonb_merge_shallow(Some(target), Some(source))
            .expect("merge should succeed");

        assert_eq!(result.0, json!({"a": 1, "b": 2, "c": 3}));
    }

    #[pgrx::pg_test]
    fn test_overlapping_keys() {
        let target = JsonB(json!({"a": 1, "b": 2}));
        let source = JsonB(json!({"b": 99, "c": 3}));

        let result = crate::jsonb_merge_shallow(Some(target), Some(source))
            .expect("merge should succeed");

        // Source value (99) should overwrite target value (2)
        assert_eq!(result.0, json!({"a": 1, "b": 99, "c": 3}));
    }

    #[pgrx::pg_test]
    fn test_empty_source() {
        let target = JsonB(json!({"a": 1}));
        let source = JsonB(json!({}));

        let result = crate::jsonb_merge_shallow(Some(target), Some(source))
            .expect("merge should succeed");

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
}
