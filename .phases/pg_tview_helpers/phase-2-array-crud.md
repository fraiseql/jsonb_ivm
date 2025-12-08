# Phase 2: Array CRUD Operations

**Duration:** 1 week (5 days)
**Priority:** 🔴 Critical
**Dependencies:** Phase 1 (uses loop unrolling patterns)
**Target Version:** v0.3.0

---

## 🎯 Objective

Implement `jsonb_array_delete_where()` and `jsonb_array_insert_where()` to complete JSONB array CRUD operations. These functions eliminate the need for re-aggregation on INSERT/DELETE, providing **3-5× speedup**.

**Current State:**
- ✅ CREATE: `jsonb_build_object()` (PostgreSQL native)
- ✅ READ: `jsonb_extract_path()` (PostgreSQL native)
- ✅ UPDATE: `jsonb_array_update_where()` (jsonb_ivm v0.2.0)
- ❌ DELETE: **Missing** (requires re-aggregation)
- ❌ INSERT: **Missing** (requires re-aggregation)

**After This Phase:**
- ✅ DELETE: `jsonb_array_delete_where()` - surgical deletion
- ✅ INSERT: `jsonb_array_insert_where()` - ordered insertion

---

## 📦 Deliverables

### 1. New Module: `src/array_crud.rs`
- [x] `jsonb_array_delete_where()` function
- [x] `jsonb_array_insert_where()` function
- [x] Helper: `find_insertion_point()` for ordered inserts
- [x] Comprehensive doc comments

### 2. Tests: `src/array_crud.rs` (test module)
- [x] DELETE: Basic deletion, no match, multiple elements
- [x] INSERT: Append, ordered insert (ASC/DESC), empty array
- [x] Edge cases: Empty arrays, non-existent paths, invalid types
- [x] Performance tests vs. re-aggregation

### 3. Integration: `src/lib.rs`
- [x] Export both functions
- [x] Add to module documentation

### 4. Benchmarks: `test/benchmark_array_crud.sql`
- [x] DELETE performance vs. re-aggregation
- [x] INSERT performance vs. re-aggregation
- [x] Stress test (100 operations)

---

## 🏗️ Implementation Design

### Function 1: `jsonb_array_delete_where()`

#### Signature
```rust
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_delete_where(
    target: JsonB,
    array_path: &str,
    match_key: &str,
    match_value: JsonB,
) -> JsonB
```

**Parameters:**
- `target` (jsonb) - JSONB document containing the array
- `array_path` (text) - Path to the array (e.g., `'posts'`)
- `match_key` (text) - Key to match on (e.g., `'id'`)
- `match_value` (jsonb) - Value to match for deletion

**Returns:** JSONB document with element removed (or unchanged if no match)

#### Implementation
```rust
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_delete_where(
    target: JsonB,
    array_path: &str,
    match_key: &str,
    match_value: JsonB,
) -> JsonB {
    let mut target_value: Value = target.0;

    // Navigate to array location
    let array = match target_value.get_mut(array_path) {
        Some(arr) => arr,
        None => return JsonB(target_value), // Array doesn't exist, return unchanged
    };

    // Validate it's an array
    let array_items = match array.as_array_mut() {
        Some(arr) => arr,
        None => return JsonB(target_value), // Not an array, return unchanged
    };

    let match_val = match_value.0;

    // Find and remove matching element
    if let Some(int_id) = match_val.as_i64() {
        // Optimized path for integer IDs (use existing helper)
        if let Some(idx) = crate::find_by_int_id_optimized(array_items, match_key, int_id) {
            array_items.remove(idx);
        }
    } else {
        // Generic path for non-integer matches
        if let Some(idx) = array_items.iter().position(|elem| {
            elem.get(match_key).map(|v| v == &match_val).unwrap_or(false)
        }) {
            array_items.remove(idx);
        }
    }

    JsonB(target_value)
}
```

#### Behavior
```sql
-- Delete post with id=2 from array
SELECT jsonb_array_delete_where(
    '{"posts": [
        {"id": 1, "title": "First"},
        {"id": 2, "title": "Second"},
        {"id": 3, "title": "Third"}
    ]}'::jsonb,
    'posts',
    'id',
    '2'::jsonb
);

-- Result: {"posts": [{"id": 1, "title": "First"}, {"id": 3, "title": "Third"}]}
```

---

### Function 2: `jsonb_array_insert_where()`

#### Signature
```rust
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_insert_where(
    target: JsonB,
    array_path: &str,
    new_element: JsonB,
    sort_key: Option<default!(&str, "NULL")>,
    sort_order: default!(&str, "'ASC'"),
) -> JsonB
```

**Parameters:**
- `target` (jsonb) - JSONB document containing (or to contain) the array
- `array_path` (text) - Path to the array (e.g., `'posts'`)
- `new_element` (jsonb) - Element to insert
- `sort_key` (text, optional) - Key to maintain sort order (e.g., `'created_at'`)
- `sort_order` (text, default 'ASC') - 'ASC' or 'DESC'

**Returns:** JSONB document with element inserted

#### Implementation
```rust
use serde_json::Value;

#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_insert_where(
    target: JsonB,
    array_path: &str,
    new_element: JsonB,
    sort_key: Option<default!(&str, "NULL")>,
    sort_order: default!(&str, "'ASC'"),
) -> JsonB {
    let mut target_value: Value = target.0;
    let new_elem = new_element.0;

    // Get or create array at path
    let target_obj = target_value.as_object_mut()
        .expect("target must be a JSONB object");

    let array = target_obj
        .entry(array_path.to_string())
        .or_insert_with(|| Value::Array(vec![]));

    let array_items = array.as_array_mut()
        .expect("path must point to an array or not exist");

    if let Some(key) = sort_key {
        // Find insertion point to maintain sort order
        let new_sort_val = new_elem.get(key);
        let insert_pos = find_insertion_point(array_items, new_sort_val, key, sort_order);
        array_items.insert(insert_pos, new_elem);
    } else {
        // No sort - append to end
        array_items.push(new_elem);
    }

    JsonB(target_value)
}

/// Find the insertion point to maintain sort order
fn find_insertion_point(
    array: &[Value],
    new_val: Option<&Value>,
    sort_key: &str,
    sort_order: &str,
) -> usize {
    let new_val = match new_val {
        Some(v) => v,
        None => return array.len(), // No sort value, insert at end
    };

    array.iter().position(|elem| {
        let elem_val = match elem.get(sort_key) {
            Some(v) => v,
            None => return false, // Element has no sort key, continue searching
        };

        // Compare values based on sort order
        if sort_order.eq_ignore_ascii_case("ASC") {
            compare_values(new_val, elem_val) == std::cmp::Ordering::Less
        } else {
            compare_values(new_val, elem_val) == std::cmp::Ordering::Greater
        }
    }).unwrap_or(array.len())
}

/// Compare two JSON values for ordering
fn compare_values(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (a, b) {
        // Numbers
        (Value::Number(a_num), Value::Number(b_num)) => {
            let a_f64 = a_num.as_f64().unwrap_or(0.0);
            let b_f64 = b_num.as_f64().unwrap_or(0.0);
            a_f64.partial_cmp(&b_f64).unwrap_or(Ordering::Equal)
        },
        // Strings (includes timestamps)
        (Value::String(a_str), Value::String(b_str)) => a_str.cmp(b_str),
        // Booleans
        (Value::Bool(a_bool), Value::Bool(b_bool)) => a_bool.cmp(b_bool),
        // Mixed types - define a consistent ordering
        (Value::Null, _) => Ordering::Less,
        (_, Value::Null) => Ordering::Greater,
        (Value::Bool(_), _) => Ordering::Less,
        (_, Value::Bool(_)) => Ordering::Greater,
        (Value::Number(_), _) => Ordering::Less,
        (_, Value::Number(_)) => Ordering::Greater,
        (Value::String(_), _) => Ordering::Less,
        (_, Value::String(_)) => Ordering::Greater,
        _ => Ordering::Equal,
    }
}
```

#### Behavior

**Append (no sort):**
```sql
SELECT jsonb_array_insert_where(
    '{"posts": [{"id": 1}, {"id": 2}]}'::jsonb,
    'posts',
    '{"id": 3, "title": "New Post"}'::jsonb
);
-- Result: {"posts": [{"id": 1}, {"id": 2}, {"id": 3, "title": "New Post"}]}
```

**Ordered insert (ASC):**
```sql
SELECT jsonb_array_insert_where(
    '{"posts": [
        {"id": 1, "created_at": "2025-01-01"},
        {"id": 3, "created_at": "2025-01-03"}
    ]}'::jsonb,
    'posts',
    '{"id": 2, "created_at": "2025-01-02"}'::jsonb,
    sort_key => 'created_at',
    sort_order => 'ASC'
);
-- Result: Inserts id=2 between id=1 and id=3 based on created_at
```

**Create array if doesn't exist:**
```sql
SELECT jsonb_array_insert_where(
    '{}'::jsonb,
    'posts',
    '{"id": 1}'::jsonb
);
-- Result: {"posts": [{"id": 1}]}
```

---

## ✅ Acceptance Criteria

### DELETE Function

#### 1. Basic Deletion
```sql
SELECT jsonb_array_delete_where(
    '{"items": [{"id": 1, "name": "A"}, {"id": 2, "name": "B"}]}'::jsonb,
    'items',
    'id',
    '1'::jsonb
);
-- Expected: {"items": [{"id": 2, "name": "B"}]}
```

#### 2. No Match (Unchanged)
```sql
SELECT jsonb_array_delete_where(
    '{"items": [{"id": 1}]}'::jsonb,
    'items',
    'id',
    '999'::jsonb
);
-- Expected: {"items": [{"id": 1}]} (unchanged)
```

#### 3. Delete from Multiple Elements
```sql
SELECT jsonb_array_delete_where(
    '{"items": [{"id": 1}, {"id": 2}, {"id": 3}, {"id": 4}]}'::jsonb,
    'items',
    'id',
    '3'::jsonb
);
-- Expected: {"items": [{"id": 1}, {"id": 2}, {"id": 4}]}
```

#### 4. Non-Existent Path
```sql
SELECT jsonb_array_delete_where(
    '{"other": []}'::jsonb,
    'items',
    'id',
    '1'::jsonb
);
-- Expected: {"other": []} (unchanged, array doesn't exist)
```

#### 5. String Match Value
```sql
SELECT jsonb_array_delete_where(
    '{"users": [{"username": "alice"}, {"username": "bob"}]}'::jsonb,
    'users',
    'username',
    '"alice"'::jsonb
);
-- Expected: {"users": [{"username": "bob"}]}
```

### INSERT Function

#### 1. Append (No Sort)
```sql
SELECT jsonb_array_insert_where(
    '{"items": [{"id": 1}, {"id": 2}]}'::jsonb,
    'items',
    '{"id": 3}'::jsonb
);
-- Expected: {"items": [{"id": 1}, {"id": 2}, {"id": 3}]}
```

#### 2. Ordered Insert (ASC)
```sql
SELECT jsonb_array_insert_where(
    '{"items": [{"priority": 1}, {"priority": 3}]}'::jsonb,
    'items',
    '{"priority": 2}'::jsonb,
    sort_key => 'priority',
    sort_order => 'ASC'
);
-- Expected: {"items": [{"priority": 1}, {"priority": 2}, {"priority": 3}]}
```

#### 3. Ordered Insert (DESC)
```sql
SELECT jsonb_array_insert_where(
    '{"items": [{"priority": 3}, {"priority": 1}]}'::jsonb,
    'items',
    '{"priority": 2}'::jsonb,
    sort_key => 'priority',
    sort_order => 'DESC'
);
-- Expected: {"items": [{"priority": 3}, {"priority": 2}, {"priority": 1}]}
```

#### 4. Create Array if Not Exists
```sql
SELECT jsonb_array_insert_where(
    '{}'::jsonb,
    'items',
    '{"id": 1}'::jsonb
);
-- Expected: {"items": [{"id": 1}]}
```

#### 5. Insert into Empty Array
```sql
SELECT jsonb_array_insert_where(
    '{"items": []}'::jsonb,
    'items',
    '{"id": 1}'::jsonb
);
-- Expected: {"items": [{"id": 1}]}
```

#### 6. String Sort Key (Timestamps)
```sql
SELECT jsonb_array_insert_where(
    '{"events": [
        {"timestamp": "2025-01-01T10:00:00Z"},
        {"timestamp": "2025-01-01T12:00:00Z"}
    ]}'::jsonb,
    'events',
    '{"timestamp": "2025-01-01T11:00:00Z"}'::jsonb,
    sort_key => 'timestamp',
    sort_order => 'ASC'
);
-- Expected: Inserts in middle based on timestamp
```

---

## 🧪 Testing Strategy

### Unit Tests (`src/array_crud.rs`)

```rust
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use pgrx::JsonB;
    use serde_json::json;

    // DELETE TESTS

    #[pgrx::pg_test]
    fn test_delete_basic() {
        let target = JsonB(json!({
            "items": [
                {"id": 1, "name": "A"},
                {"id": 2, "name": "B"},
                {"id": 3, "name": "C"}
            ]
        }));

        let result = crate::jsonb_array_delete_where(
            target,
            "items",
            "id",
            JsonB(json!(2)),
        );

        let expected = json!({
            "items": [
                {"id": 1, "name": "A"},
                {"id": 3, "name": "C"}
            ]
        });

        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_delete_no_match() {
        let target = JsonB(json!({"items": [{"id": 1}]}));

        let result = crate::jsonb_array_delete_where(
            target.clone(),
            "items",
            "id",
            JsonB(json!(999)),
        );

        // Should return unchanged
        assert_eq!(result.0, target.0);
    }

    #[pgrx::pg_test]
    fn test_delete_non_existent_path() {
        let target = JsonB(json!({"other": []}));

        let result = crate::jsonb_array_delete_where(
            target.clone(),
            "items",
            "id",
            JsonB(json!(1)),
        );

        // Should return unchanged
        assert_eq!(result.0, target.0);
    }

    #[pgrx::pg_test]
    fn test_delete_string_value() {
        let target = JsonB(json!({
            "users": [
                {"username": "alice"},
                {"username": "bob"}
            ]
        }));

        let result = crate::jsonb_array_delete_where(
            target,
            "users",
            "username",
            JsonB(json!("alice")),
        );

        let expected = json!({"users": [{"username": "bob"}]});
        assert_eq!(result.0, expected);
    }

    // INSERT TESTS

    #[pgrx::pg_test]
    fn test_insert_append() {
        let target = JsonB(json!({"items": [{"id": 1}, {"id": 2}]}));

        let result = crate::jsonb_array_insert_where(
            target,
            "items",
            JsonB(json!({"id": 3})),
            None,
            "ASC",
        );

        let expected = json!({"items": [{"id": 1}, {"id": 2}, {"id": 3}]});
        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_insert_ordered_asc() {
        let target = JsonB(json!({
            "items": [
                {"priority": 1},
                {"priority": 3}
            ]
        }));

        let result = crate::jsonb_array_insert_where(
            target,
            "items",
            JsonB(json!({"priority": 2})),
            Some("priority"),
            "ASC",
        );

        let expected = json!({
            "items": [
                {"priority": 1},
                {"priority": 2},
                {"priority": 3}
            ]
        });
        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_insert_ordered_desc() {
        let target = JsonB(json!({
            "items": [
                {"priority": 3},
                {"priority": 1}
            ]
        }));

        let result = crate::jsonb_array_insert_where(
            target,
            "items",
            JsonB(json!({"priority": 2})),
            Some("priority"),
            "DESC",
        );

        let expected = json!({
            "items": [
                {"priority": 3},
                {"priority": 2},
                {"priority": 1}
            ]
        });
        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_insert_create_array() {
        let target = JsonB(json!({}));

        let result = crate::jsonb_array_insert_where(
            target,
            "items",
            JsonB(json!({"id": 1})),
            None,
            "ASC",
        );

        let expected = json!({"items": [{"id": 1}]});
        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_insert_empty_array() {
        let target = JsonB(json!({"items": []}));

        let result = crate::jsonb_array_insert_where(
            target,
            "items",
            JsonB(json!({"id": 1})),
            None,
            "ASC",
        );

        let expected = json!({"items": [{"id": 1}]});
        assert_eq!(result.0, expected);
    }
}
```

---

## 📊 Benchmark Plan

Create `test/benchmark_array_crud.sql`:

```sql
-- Setup: Create test data
CREATE TABLE bench_posts (
    pk_post INT PRIMARY KEY,
    id UUID,
    title TEXT,
    created_at TIMESTAMPTZ
);

CREATE TABLE bench_tv_feed (
    pk_feed INT PRIMARY KEY,
    data JSONB
);

-- Insert 100 posts
INSERT INTO bench_posts
SELECT
    i,
    gen_random_uuid(),
    'Post ' || i,
    now() - (i || ' minutes')::interval
FROM generate_series(1, 100) i;

-- Create feed with aggregated posts
INSERT INTO bench_tv_feed (pk_feed, data)
SELECT 1, jsonb_build_object(
    'posts',
    jsonb_agg(
        jsonb_build_object(
            'id', id::text,
            'title', title,
            'created_at', created_at
        ) ORDER BY created_at DESC
    )
)
FROM bench_posts;

\timing on

-- ===== DELETE BENCHMARKS =====

-- Baseline: Re-aggregation (SLOW)
EXPLAIN ANALYZE
UPDATE bench_tv_feed
SET data = jsonb_build_object(
    'posts',
    (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', id::text,
                'title', title,
                'created_at', created_at
            ) ORDER BY created_at DESC
        )
        FROM bench_posts
        WHERE pk_post != 50  -- Delete post 50
    )
)
WHERE pk_feed = 1;
-- Expected: ~15-20ms (re-aggregates all 99 posts)

-- Rollback to restore data
ROLLBACK;
BEGIN;

-- Our implementation: Surgical deletion (FAST)
EXPLAIN ANALYZE
UPDATE bench_tv_feed
SET data = jsonb_array_delete_where(
    data,
    'posts',
    'id',
    (SELECT id::text::jsonb FROM bench_posts WHERE pk_post = 50)
)
WHERE pk_feed = 1;
-- Expected: ~3-5ms (deletes single element)
-- Target: 3-5× faster

ROLLBACK;

-- ===== INSERT BENCHMARKS =====

-- Add new post to bench_posts
INSERT INTO bench_posts VALUES (101, gen_random_uuid(), 'New Post', now());

BEGIN;

-- Baseline: Re-aggregation (SLOW)
EXPLAIN ANALYZE
UPDATE bench_tv_feed
SET data = jsonb_build_object(
    'posts',
    (
        SELECT jsonb_agg(
            jsonb_build_object(
                'id', id::text,
                'title', title,
                'created_at', created_at
            ) ORDER BY created_at DESC
        )
        FROM bench_posts
    )
)
WHERE pk_feed = 1;
-- Expected: ~15-20ms (re-aggregates all 101 posts)

ROLLBACK;
BEGIN;

-- Our implementation: Surgical insertion (FAST)
EXPLAIN ANALYZE
UPDATE bench_tv_feed
SET data = jsonb_array_insert_where(
    data,
    'posts',
    (
        SELECT jsonb_build_object(
            'id', id::text,
            'title', title,
            'created_at', created_at
        )
        FROM bench_posts WHERE pk_post = 101
    ),
    sort_key => 'created_at',
    sort_order => 'DESC'
)
WHERE pk_feed = 1;
-- Expected: ~3-5ms (inserts single element at correct position)
-- Target: 3-5× faster

ROLLBACK;

-- ===== STRESS TEST: 100 DELETES =====

BEGIN;

-- Baseline: 100 re-aggregations
SELECT clock_timestamp() AS start_time \gset
DO $$
DECLARE
    i INT;
BEGIN
    FOR i IN 1..100 LOOP
        UPDATE bench_tv_feed
        SET data = jsonb_build_object(
            'posts',
            (
                SELECT jsonb_agg(
                    jsonb_build_object(
                        'id', id::text,
                        'title', title,
                        'created_at', created_at
                    ) ORDER BY created_at DESC
                )
                FROM bench_posts
                WHERE pk_post != i
            )
        )
        WHERE pk_feed = 1;
    END LOOP;
END $$;
SELECT clock_timestamp() AS end_time \gset
SELECT :'end_time'::timestamp - :'start_time'::timestamp AS baseline_time;
-- Expected: ~1500-2000ms (15-20ms × 100)

ROLLBACK;
BEGIN;

-- Our implementation: 100 surgical deletions
SELECT clock_timestamp() AS start_time \gset
DO $$
DECLARE
    post_id UUID;
BEGIN
    FOR post_id IN (SELECT id FROM bench_posts LIMIT 100) LOOP
        UPDATE bench_tv_feed
        SET data = jsonb_array_delete_where(
            data,
            'posts',
            'id',
            post_id::text::jsonb
        )
        WHERE pk_feed = 1;
    END LOOP;
END $$;
SELECT clock_timestamp() AS end_time \gset
SELECT :'end_time'::timestamp - :'start_time'::timestamp AS optimized_time;
-- Expected: ~300-500ms (3-5ms × 100)
-- Target: 3-5× faster

ROLLBACK;

-- Cleanup
DROP TABLE bench_posts, bench_tv_feed;
```

**Success Criteria:**
- DELETE: 3-5× faster than re-aggregation
- INSERT: 3-5× faster than re-aggregation
- Stress test: 3-5× faster for 100 operations

---

## 📝 Step-by-Step Implementation

### Day 1: DELETE Function
1. Create `src/array_crud.rs`
2. Implement `jsonb_array_delete_where()` core logic
3. Add integer ID optimization path
4. Test basic deletion

### Day 2: INSERT Function (Part 1)
1. Implement `jsonb_array_insert_where()` append case
2. Implement array creation if doesn't exist
3. Test append functionality

### Day 3: INSERT Function (Part 2)
1. Implement `find_insertion_point()` helper
2. Implement `compare_values()` helper
3. Add ordered insertion logic (ASC/DESC)
4. Test ordered insertion

### Day 4: Testing
1. Write comprehensive unit tests (DELETE)
2. Write comprehensive unit tests (INSERT)
3. Test edge cases (empty arrays, non-existent paths)
4. All tests passing

### Day 5: Benchmarks & Documentation
1. Write benchmark SQL file
2. Run benchmarks and document results
3. Write doc comments for both functions
4. Update README.md and CHANGELOG.md
5. Code review

---

## ✅ Phase Completion Checklist

### Implementation
- [ ] Module created: `src/array_crud.rs`
- [ ] `jsonb_array_delete_where()` implemented
- [ ] `jsonb_array_insert_where()` implemented
- [ ] Helper functions implemented
- [ ] Exported from `lib.rs`

### Testing
- [ ] DELETE unit tests (5+ test cases)
- [ ] INSERT unit tests (6+ test cases)
- [ ] Edge case tests
- [ ] All tests passing

### Performance
- [ ] Benchmark file created
- [ ] DELETE: 3-5× speedup validated
- [ ] INSERT: 3-5× speedup validated
- [ ] Stress test: 100 operations validated

### Documentation
- [ ] Doc comments complete
- [ ] SQL examples provided
- [ ] README.md updated
- [ ] CHANGELOG.md updated

### Quality
- [ ] No compiler warnings
- [ ] No clippy warnings
- [ ] Code style consistent
- [ ] Self-review completed

---

**Next Phase:** [Phase 3: Deep Merge & Helpers](phase-3-deep-merge.md)
