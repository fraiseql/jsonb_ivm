# Phase 3: Deep Merge & Helpers

**Duration:** 1 week (5 days)
**Priority:** 🟡 High
**Dependencies:** Phase 1, Phase 2
**Target Version:** v0.3.0

---

## 🎯 Objective

Implement `jsonb_deep_merge()` for recursive JSONB merging and helper utilities (`jsonb_extract_id`, `jsonb_array_contains_id`) to simplify pg_tview implementation. These functions handle complex nested scenarios and provide convenient utilities for common TVIEW patterns.

---

## 📦 Deliverables

### 1. New Module: `src/deep_merge.rs`
- [x] `jsonb_deep_merge()` function
- [x] `deep_merge_recursive()` helper (internal)
- [x] Comprehensive doc comments

### 2. New Module: `src/helpers.rs`
- [x] `jsonb_extract_id()` - Safe ID extraction
- [x] `jsonb_array_contains_id()` - Fast containment check
- [x] Doc comments with examples

### 3. Tests
- [x] `src/deep_merge.rs` test module
- [x] `src/helpers.rs` test module
- [x] Edge cases and error handling

### 4. Integration: `src/lib.rs`
- [x] Export all three functions
- [x] Update module documentation

---

## 🏗️ Implementation Design

### Function 1: `jsonb_deep_merge()`

#### Problem Statement

**Current behavior (jsonb_merge_shallow):**
```sql
SELECT jsonb_merge_shallow(
    '{"user": {"name": "Alice", "company": {"name": "ACME", "city": "NYC"}}}'::jsonb,
    '{"user": {"company": {"name": "ACME Corp"}}}'::jsonb
);
-- Result: {"user": {"company": {"name": "ACME Corp"}}}
-- ❌ Lost "name": "Alice" and "city": "NYC"
```

**Desired behavior (jsonb_deep_merge):**
```sql
SELECT jsonb_deep_merge(
    '{"user": {"name": "Alice", "company": {"name": "ACME", "city": "NYC"}}}'::jsonb,
    '{"user": {"company": {"name": "ACME Corp"}}}'::jsonb
);
-- Result: {"user": {"name": "Alice", "company": {"name": "ACME Corp", "city": "NYC"}}}
-- ✅ Preserves all sibling fields
```

#### Signature

```rust
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_deep_merge(target: JsonB, source: JsonB) -> JsonB
```

**Parameters:**
- `target` (jsonb) - Base JSONB document
- `source` (jsonb) - JSONB to merge (recursively)

**Returns:** Deeply merged JSONB

#### Implementation

```rust
use pgrx::prelude::*;
use pgrx::JsonB;
use serde_json::Value;

#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_deep_merge(target: JsonB, source: JsonB) -> JsonB {
    let target_val = target.0;
    let source_val = source.0;

    JsonB(deep_merge_recursive(target_val, source_val))
}

/// Recursively merge two JSON values
/// If both are objects, recursively merge their keys
/// Otherwise, source value replaces target value
fn deep_merge_recursive(mut target: Value, source: Value) -> Value {
    // If both are objects, merge recursively
    if let (Some(target_obj), Some(source_obj)) = (target.as_object_mut(), source.as_object()) {
        for (key, source_value) in source_obj {
            target_obj
                .entry(key.clone())
                .and_modify(|target_value| {
                    // Recursively merge if both are objects
                    *target_value = deep_merge_recursive(target_value.clone(), source_value.clone());
                })
                .or_insert_with(|| source_value.clone());
        }
        target
    } else {
        // If not both objects, source wins (replaces target)
        source
    }
}
```

#### Behavior Examples

**Simple nested merge:**
```sql
SELECT jsonb_deep_merge(
    '{"a": {"b": 1, "c": 2}}'::jsonb,
    '{"a": {"c": 3, "d": 4}}'::jsonb
);
-- Result: {"a": {"b": 1, "c": 3, "d": 4}}
```

**Deep nested merge (3 levels):**
```sql
SELECT jsonb_deep_merge(
    '{"level1": {"level2": {"level3": {"a": 1, "b": 2}}}}'::jsonb,
    '{"level1": {"level2": {"level3": {"b": 99, "c": 3}}}}'::jsonb
);
-- Result: {"level1": {"level2": {"level3": {"a": 1, "b": 99, "c": 3}}}}
```

**Array replacement (arrays are not merged):**
```sql
SELECT jsonb_deep_merge(
    '{"items": [1, 2, 3]}'::jsonb,
    '{"items": [4, 5]}'::jsonb
);
-- Result: {"items": [4, 5]}  (source array replaces target array)
```

---

### Function 2: `jsonb_extract_id()`

#### Problem Statement

pg_tview frequently needs to extract ID values from JSONB for propagation logic:

```rust
// Current approach (error-prone):
let id_value = new_data.0.get("id")
    .and_then(|v| v.as_str())
    .ok_or(Error::MissingId)?;
```

**Better approach:**
```sql
SELECT jsonb_extract_id('{"id": "550e8400-...", "name": "Alice"}'::jsonb);
-- Returns: '550e8400-...'
```

#### Signature

```rust
#[pg_extern(immutable, parallel_safe)]
fn jsonb_extract_id(
    data: JsonB,
    key: default!(&str, "'id'"),
) -> Option<String>
```

**Parameters:**
- `data` (jsonb) - JSONB document
- `key` (text, default 'id') - Key to extract

**Returns:** ID value as text (NULL if not found)

#### Implementation

```rust
use pgrx::prelude::*;
use pgrx::JsonB;

#[pg_extern(immutable, parallel_safe)]
fn jsonb_extract_id(data: JsonB, key: default!(&str, "'id'")) -> Option<String> {
    let obj = data.0.as_object()?;
    let id_value = obj.get(key)?;

    match id_value {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}
```

#### Behavior Examples

```sql
-- UUID extraction
SELECT jsonb_extract_id('{"id": "550e8400-e29b-41d4-a716-446655440000", "name": "Alice"}'::jsonb);
-- Returns: '550e8400-e29b-41d4-a716-446655440000'

-- Integer extraction
SELECT jsonb_extract_id('{"id": 42, "title": "Post"}'::jsonb);
-- Returns: '42'

-- Custom key
SELECT jsonb_extract_id('{"post_id": 123, "title": "..."}'::jsonb, 'post_id');
-- Returns: '123'

-- Not found
SELECT jsonb_extract_id('{"name": "Alice"}'::jsonb);
-- Returns: NULL

-- Invalid type (boolean)
SELECT jsonb_extract_id('{"id": true}'::jsonb);
-- Returns: NULL
```

---

### Function 3: `jsonb_array_contains_id()`

#### Problem Statement

pg_tview needs to check if a JSONB array contains an element with a specific ID:

```sql
-- Current approach (slow):
SELECT pk_feed FROM tv_feed
WHERE data->'posts' @> '[{"id": "550e8400-..."}]'::jsonb;
-- Problem: @> operator checks subset, not exact match on id field
```

**Better approach:**
```sql
SELECT pk_feed FROM tv_feed
WHERE jsonb_array_contains_id(data, 'posts', 'id', '550e8400-...'::jsonb);
-- Uses optimized loop unrolling for integer IDs
```

#### Signature

```rust
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_contains_id(
    data: JsonB,
    array_path: &str,
    id_key: &str,
    id_value: JsonB,
) -> bool
```

**Parameters:**
- `data` (jsonb) - JSONB document containing array
- `array_path` (text) - Path to array (e.g., 'posts')
- `id_key` (text) - Key to match on (e.g., 'id')
- `id_value` (jsonb) - Value to search for

**Returns:** true if element found, false otherwise

#### Implementation

```rust
use pgrx::prelude::*;
use pgrx::JsonB;

#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_contains_id(
    data: JsonB,
    array_path: &str,
    id_key: &str,
    id_value: JsonB,
) -> bool {
    let obj = match data.0.as_object() {
        Some(o) => o,
        None => return false,
    };

    let array = match obj.get(array_path).and_then(|v| v.as_array()) {
        Some(arr) => arr,
        None => return false,
    };

    // Use optimized search if ID is integer
    if let Some(int_id) = id_value.0.as_i64() {
        crate::find_by_int_id_optimized(array, id_key, int_id).is_some()
    } else {
        // Generic search for non-integer IDs
        array.iter().any(|elem| {
            elem.get(id_key).map(|v| v == &id_value.0).unwrap_or(false)
        })
    }
}
```

#### Behavior Examples

```sql
-- Integer ID (uses optimized search)
SELECT jsonb_array_contains_id(
    '{"posts": [{"id": 1}, {"id": 2}, {"id": 3}]}'::jsonb,
    'posts',
    'id',
    '2'::jsonb
);
-- Returns: true

-- UUID (generic search)
SELECT jsonb_array_contains_id(
    '{"posts": [{"id": "550e8400-..."}, {"id": "660f9500-..."}]}'::jsonb,
    'posts',
    'id',
    '"550e8400-..."'::jsonb
);
-- Returns: true

-- Not found
SELECT jsonb_array_contains_id(
    '{"posts": [{"id": 1}, {"id": 2}]}'::jsonb,
    'posts',
    'id',
    '999'::jsonb
);
-- Returns: false

-- Array doesn't exist
SELECT jsonb_array_contains_id(
    '{"other": []}'::jsonb,
    'posts',
    'id',
    '1'::jsonb
);
-- Returns: false
```

---

## ✅ Acceptance Criteria

### Deep Merge

#### 1. Simple Nested Merge
```sql
SELECT jsonb_deep_merge(
    '{"a": {"b": 1, "c": 2}}'::jsonb,
    '{"a": {"c": 3, "d": 4}}'::jsonb
);
-- Expected: {"a": {"b": 1, "c": 3, "d": 4}}
```

#### 2. Deep Nested Merge (3+ levels)
```sql
SELECT jsonb_deep_merge(
    '{"l1": {"l2": {"l3": {"a": 1}}}}'::jsonb,
    '{"l1": {"l2": {"l3": {"b": 2}}}}'::jsonb
);
-- Expected: {"l1": {"l2": {"l3": {"a": 1, "b": 2}}}}
```

#### 3. Array Replacement (Not Merged)
```sql
SELECT jsonb_deep_merge(
    '{"arr": [1, 2]}'::jsonb,
    '{"arr": [3, 4]}'::jsonb
);
-- Expected: {"arr": [3, 4]}  (source replaces target)
```

#### 4. Mixed Types
```sql
SELECT jsonb_deep_merge(
    '{"a": {"b": 1}}'::jsonb,
    '{"a": "replaced"}'::jsonb
);
-- Expected: {"a": "replaced"}  (source replaces target)
```

### Extract ID

#### 1. UUID Extraction
```sql
SELECT jsonb_extract_id('{"id": "550e8400-...", "name": "..."}'::jsonb);
-- Expected: '550e8400-...'
```

#### 2. Integer Extraction
```sql
SELECT jsonb_extract_id('{"id": 42}'::jsonb);
-- Expected: '42'
```

#### 3. Custom Key
```sql
SELECT jsonb_extract_id('{"post_id": 123}'::jsonb, 'post_id');
-- Expected: '123'
```

#### 4. Not Found
```sql
SELECT jsonb_extract_id('{"name": "Alice"}'::jsonb);
-- Expected: NULL
```

### Array Contains ID

#### 1. Integer ID (Optimized Path)
```sql
SELECT jsonb_array_contains_id(
    '{"items": [{"id": 1}, {"id": 2}]}'::jsonb,
    'items', 'id', '1'::jsonb
);
-- Expected: true
```

#### 2. UUID (Generic Path)
```sql
SELECT jsonb_array_contains_id(
    '{"items": [{"id": "550e8400-..."}]}'::jsonb,
    'items', 'id', '"550e8400-..."'::jsonb
);
-- Expected: true
```

#### 3. Not Found
```sql
SELECT jsonb_array_contains_id(
    '{"items": [{"id": 1}]}'::jsonb,
    'items', 'id', '999'::jsonb
);
-- Expected: false
```

---

## 🧪 Testing Strategy

### Deep Merge Tests

```rust
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use pgrx::JsonB;
    use serde_json::json;

    #[pgrx::pg_test]
    fn test_deep_merge_simple() {
        let target = JsonB(json!({"a": {"b": 1, "c": 2}}));
        let source = JsonB(json!({"a": {"c": 3, "d": 4}}));

        let result = crate::jsonb_deep_merge(target, source);

        assert_eq!(result.0, json!({"a": {"b": 1, "c": 3, "d": 4}}));
    }

    #[pgrx::pg_test]
    fn test_deep_merge_nested() {
        let target = JsonB(json!({
            "level1": {
                "level2": {
                    "level3": {"a": 1, "b": 2}
                }
            }
        }));
        let source = JsonB(json!({
            "level1": {
                "level2": {
                    "level3": {"b": 99, "c": 3}
                }
            }
        }));

        let result = crate::jsonb_deep_merge(target, source);

        let expected = json!({
            "level1": {
                "level2": {
                    "level3": {"a": 1, "b": 99, "c": 3}
                }
            }
        });

        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_deep_merge_array_replacement() {
        let target = JsonB(json!({"items": [1, 2, 3]}));
        let source = JsonB(json!({"items": [4, 5]}));

        let result = crate::jsonb_deep_merge(target, source);

        // Arrays are replaced, not merged
        assert_eq!(result.0, json!({"items": [4, 5]}));
    }
}
```

### Helper Tests

```rust
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use pgrx::JsonB;
    use serde_json::json;

    // EXTRACT ID TESTS

    #[pgrx::pg_test]
    fn test_extract_id_uuid() {
        let data = JsonB(json!({"id": "550e8400-e29b-41d4-a716-446655440000"}));
        let result = crate::jsonb_extract_id(data, "id");
        assert_eq!(result, Some("550e8400-e29b-41d4-a716-446655440000".to_string()));
    }

    #[pgrx::pg_test]
    fn test_extract_id_integer() {
        let data = JsonB(json!({"id": 42}));
        let result = crate::jsonb_extract_id(data, "id");
        assert_eq!(result, Some("42".to_string()));
    }

    #[pgrx::pg_test]
    fn test_extract_id_custom_key() {
        let data = JsonB(json!({"post_id": 123}));
        let result = crate::jsonb_extract_id(data, "post_id");
        assert_eq!(result, Some("123".to_string()));
    }

    #[pgrx::pg_test]
    fn test_extract_id_not_found() {
        let data = JsonB(json!({"name": "Alice"}));
        let result = crate::jsonb_extract_id(data, "id");
        assert_eq!(result, None);
    }

    // ARRAY CONTAINS ID TESTS

    #[pgrx::pg_test]
    fn test_contains_id_integer() {
        let data = JsonB(json!({"items": [{"id": 1}, {"id": 2}]}));
        let result = crate::jsonb_array_contains_id(
            data,
            "items",
            "id",
            JsonB(json!(1)),
        );
        assert!(result);
    }

    #[pgrx::pg_test]
    fn test_contains_id_not_found() {
        let data = JsonB(json!({"items": [{"id": 1}, {"id": 2}]}));
        let result = crate::jsonb_array_contains_id(
            data,
            "items",
            "id",
            JsonB(json!(999)),
        );
        assert!(!result);
    }

    #[pgrx::pg_test]
    fn test_contains_id_array_not_exist() {
        let data = JsonB(json!({"other": []}));
        let result = crate::jsonb_array_contains_id(
            data,
            "items",
            "id",
            JsonB(json!(1)),
        );
        assert!(!result);
    }
}
```

---

## 📊 Performance Considerations

### Deep Merge Performance
- **Complexity:** O(n × depth) where n = number of keys, depth = max nesting level
- **Typical TVIEW use case:** depth ≤ 3, n ≤ 20 keys → ~0.5-1ms
- **Target:** < 2ms for depth ≤ 5, < 100 keys

### Contains ID Performance
- **Integer IDs:** O(n) with loop unrolling optimization (~100ns per element)
- **Non-integer IDs:** O(n) generic search (~200ns per element)
- **Typical TVIEW use case:** n ≤ 100 elements → ~0.01-0.02ms

---

## 📝 Step-by-Step Implementation

### Day 1: Deep Merge
1. Create `src/deep_merge.rs`
2. Implement `deep_merge_recursive()` helper
3. Implement `jsonb_deep_merge()` wrapper
4. Basic tests

### Day 2: Helpers (Part 1)
1. Create `src/helpers.rs`
2. Implement `jsonb_extract_id()`
3. Unit tests for extract_id

### Day 3: Helpers (Part 2)
1. Implement `jsonb_array_contains_id()`
2. Unit tests for contains_id
3. Edge case testing

### Day 4: Integration & Testing
1. Export functions from `lib.rs`
2. Comprehensive unit tests
3. Integration tests (SQL patterns)
4. All tests passing

### Day 5: Documentation & Polish
1. Doc comments with examples
2. Update README.md
3. Update CHANGELOG.md
4. Code review

---

## ✅ Phase Completion Checklist

### Implementation
- [ ] `src/deep_merge.rs` created
- [ ] `src/helpers.rs` created
- [ ] All three functions implemented
- [ ] Exported from `lib.rs`

### Testing
- [ ] Deep merge tests (4+ cases)
- [ ] Extract ID tests (4+ cases)
- [ ] Contains ID tests (3+ cases)
- [ ] All tests passing

### Documentation
- [ ] Doc comments complete
- [ ] SQL examples provided
- [ ] README.md updated
- [ ] CHANGELOG.md updated

### Quality
- [ ] No warnings
- [ ] Code style consistent
- [ ] Performance acceptable
- [ ] Self-review completed

---

**Next Phase:** [Phase 4: Integration & Benchmarks](phase-4-integration.md)
