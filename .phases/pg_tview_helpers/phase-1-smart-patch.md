# Phase 1: Smart Patch Dispatcher

**Duration:** 1 week (5 days)
**Priority:** 🔴 Critical
**Dependencies:** None (uses existing functions)
**Target Version:** v0.3.0

---

## 🎯 Objective

Implement `jsonb_smart_patch()` - an intelligent dispatcher that routes JSONB updates to the optimal function based on metadata. This is the **highest impact** feature for pg_tview, reducing complexity by 60%.

---

## 📦 Deliverables

### 1. Implementation: `src/lib.rs`
- [x] `jsonb_smart_patch_scalar()` function implementation
- [x] `jsonb_smart_patch_nested()` function implementation
- [x] `jsonb_smart_patch_array()` function implementation
- [x] Comprehensive doc comments with pg_tview examples

### 2. Tests: `test/sql/04_smart_patch.sql`
- [x] Test all three function types (scalar, nested, array)
- [x] Test with various data types (int IDs, string IDs)
- [x] Test edge cases (no match, multiple fields)
- [x] Test pg_tview integration patterns (12 tests total)

### 3. SQL Schema: Manual creation (pgrx limitation workaround)
- [x] Created `jsonb_ivm--0.2.0.sql` manually
- [x] All 3 functions exported correctly

### 4. Documentation
- [x] Function doc comments with SQL examples
- [x] pg_tview usage patterns documented
- [ ] Update README.md with new functions
- [ ] Add to CHANGELOG.md

**Note:** Original plan was for single `jsonb_smart_patch()` dispatcher function, but split into 3 separate functions due to pgrx limitation with `Option<pgrx::Array<&str>>` parameters. This approach is actually cleaner and provides better type safety.

---

## 🏗️ Implementation Design

### Function Signature

```rust
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_smart_patch(
    target: JsonB,
    source: JsonB,
    patch_type: &str,
    path: Option<default!(pgrx::Array<&str>, "NULL")>,
    array_match_key: Option<default!(&str, "NULL")>,
    match_value: Option<default!(JsonB, "NULL")>,
) -> JsonB
```

**Parameters:**
- `target` (jsonb) - Current JSONB document
- `source` (jsonb) - New data to merge
- `patch_type` (text) - One of: 'scalar', 'nested_object', 'array'
- `path` (text[], optional) - JSONB path for nested/array updates
- `array_match_key` (text, optional) - Match key for array updates
- `match_value` (jsonb, optional) - Match value for array updates

**Returns:** Updated JSONB document

---

### Implementation Logic

```rust
use pgrx::prelude::*;
use pgrx::JsonB;

#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_smart_patch(
    target: JsonB,
    source: JsonB,
    patch_type: &str,
    path: Option<default!(pgrx::Array<&str>, "NULL")>,
    array_match_key: Option<default!(&str, "NULL")>,
    match_value: Option<default!(JsonB, "NULL")>,
) -> JsonB {
    match patch_type {
        "scalar" => {
            // Use jsonb_merge_shallow for root-level merge
            crate::jsonb_merge_shallow(Some(target), Some(source))
                .expect("jsonb_merge_shallow should not return NULL with valid inputs")
        },

        "nested_object" => {
            // Validate path is provided
            let path_array = match path {
                Some(p) => p,
                None => error!("path parameter is required for patch_type='nested_object'"),
            };

            // Use jsonb_merge_at_path for nested updates
            crate::jsonb_merge_at_path(target, source, path_array)
        },

        "array" => {
            // Validate all required parameters
            let path_array = match path {
                Some(p) => p,
                None => error!("path parameter is required for patch_type='array'"),
            };

            let match_key = match array_match_key {
                Some(k) => k,
                None => error!("array_match_key parameter is required for patch_type='array'"),
            };

            let match_val = match match_value {
                Some(v) => v,
                None => error!("match_value parameter is required for patch_type='array'"),
            };

            // Extract first element of path as array_path
            let array_path = path_array
                .iter()
                .next()
                .flatten()
                .expect("path must have at least one element for array updates");

            // Use jsonb_array_update_where for array element updates
            crate::jsonb_array_update_where(target, array_path, match_key, match_val, source)
        },

        _ => error!(
            "Invalid patch_type: '{}'. Must be 'scalar', 'nested_object', or 'array'",
            patch_type
        ),
    }
}
```

---

## ✅ Acceptance Criteria

### Functional Requirements

#### 1. Scalar Update (Root-Level Merge)
```sql
SELECT jsonb_smart_patch(
    '{"id": 1, "name": "old", "count": 10}'::jsonb,
    '{"name": "new", "active": true}'::jsonb,
    'scalar'
);
-- Expected: {"id": 1, "name": "new", "count": 10, "active": true}
```

#### 2. Nested Object Update
```sql
SELECT jsonb_smart_patch(
    '{"id": 1, "user": {"name": "Alice", "company": {"name": "ACME", "city": "NYC"}}}'::jsonb,
    '{"name": "ACME Corp"}'::jsonb,
    'nested_object',
    path => ARRAY['user', 'company']
);
-- Expected: {"id": 1, "user": {"name": "Alice", "company": {"name": "ACME Corp", "city": "NYC"}}}
```

#### 3. Array Element Update
```sql
SELECT jsonb_smart_patch(
    '{"posts": [{"id": 1, "title": "Old"}, {"id": 2, "title": "Post 2"}]}'::jsonb,
    '{"title": "New", "updated": true}'::jsonb,
    'array',
    path => ARRAY['posts'],
    array_match_key => 'id',
    match_value => '1'::jsonb
);
-- Expected: {"posts": [{"id": 1, "title": "New", "updated": true}, {"id": 2, "title": "Post 2"}]}
```

#### 4. Error Handling
```sql
-- Invalid patch_type
SELECT jsonb_smart_patch('...'::jsonb, '...'::jsonb, 'invalid_type');
-- Expected: ERROR: Invalid patch_type: 'invalid_type'

-- Missing required parameter (nested_object without path)
SELECT jsonb_smart_patch('...'::jsonb, '...'::jsonb, 'nested_object');
-- Expected: ERROR: path parameter is required for patch_type='nested_object'

-- Missing array_match_key
SELECT jsonb_smart_patch('...'::jsonb, '...'::jsonb, 'array', path => ARRAY['items']);
-- Expected: ERROR: array_match_key parameter is required for patch_type='array'
```

### Non-Functional Requirements

1. **Performance:** Dispatch overhead < 0.1ms (measured via benchmark)
2. **Memory:** No memory leaks (test with valgrind if available)
3. **Compatibility:** Works with PostgreSQL 15, 16, 17
4. **Tests:** 100% code coverage for all branches

---

## 🧪 Testing Strategy

### Unit Tests (in `src/smart_patch.rs`)

```rust
#[cfg(any(test, feature = "pg_test"))]
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;
    use pgrx::JsonB;
    use serde_json::json;

    #[pgrx::pg_test]
    fn test_smart_patch_scalar() {
        let target = JsonB(json!({"id": 1, "name": "old"}));
        let source = JsonB(json!({"name": "new", "active": true}));

        let result = crate::jsonb_smart_patch(
            target,
            source,
            "scalar",
            None,
            None,
            None,
        );

        assert_eq!(result.0, json!({"id": 1, "name": "new", "active": true}));
    }

    #[pgrx::pg_test]
    fn test_smart_patch_nested_object() {
        let target = JsonB(json!({
            "id": 1,
            "user": {
                "name": "Alice",
                "company": {"name": "ACME", "city": "NYC"}
            }
        }));
        let source = JsonB(json!({"name": "ACME Corp"}));
        let path = pgrx::Array::from(vec!["user", "company"]);

        let result = crate::jsonb_smart_patch(
            target,
            source,
            "nested_object",
            Some(path),
            None,
            None,
        );

        let expected = json!({
            "id": 1,
            "user": {
                "name": "Alice",
                "company": {"name": "ACME Corp", "city": "NYC"}
            }
        });

        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    fn test_smart_patch_array() {
        let target = JsonB(json!({
            "posts": [
                {"id": 1, "title": "Old"},
                {"id": 2, "title": "Post 2"}
            ]
        }));
        let source = JsonB(json!({"title": "New", "updated": true}));
        let path = pgrx::Array::from(vec!["posts"]);
        let match_value = JsonB(json!(1));

        let result = crate::jsonb_smart_patch(
            target,
            source,
            "array",
            Some(path),
            Some("id"),
            Some(match_value),
        );

        let expected = json!({
            "posts": [
                {"id": 1, "title": "New", "updated": true},
                {"id": 2, "title": "Post 2"}
            ]
        });

        assert_eq!(result.0, expected);
    }

    #[pgrx::pg_test]
    #[should_panic(expected = "Invalid patch_type")]
    fn test_smart_patch_invalid_type() {
        let target = JsonB(json!({"id": 1}));
        let source = JsonB(json!({"name": "test"}));

        let _ = crate::jsonb_smart_patch(
            target,
            source,
            "invalid_type",
            None,
            None,
            None,
        );
    }

    #[pgrx::pg_test]
    #[should_panic(expected = "path parameter is required")]
    fn test_smart_patch_missing_path_nested() {
        let target = JsonB(json!({"user": {"name": "Alice"}}));
        let source = JsonB(json!({"name": "Bob"}));

        let _ = crate::jsonb_smart_patch(
            target,
            source,
            "nested_object",
            None,
            None,
            None,
        );
    }

    #[pgrx::pg_test]
    #[should_panic(expected = "array_match_key parameter is required")]
    fn test_smart_patch_missing_array_match_key() {
        let target = JsonB(json!({"items": [{"id": 1}]}));
        let source = JsonB(json!({"name": "test"}));
        let path = pgrx::Array::from(vec!["items"]);

        let _ = crate::jsonb_smart_patch(
            target,
            source,
            "array",
            Some(path),
            None,
            None,
        );
    }
}
```

### Integration Tests (SQL)

Create `test/smart_patch_integration.sql`:

```sql
-- Test 1: pg_tview pattern - scalar update
CREATE TABLE test_tv_company (
    pk_company INT PRIMARY KEY,
    id UUID,
    data JSONB
);

INSERT INTO test_tv_company VALUES
    (1, '550e8400-e29b-41d4-a716-446655440000'::uuid, '{"id": "550e8400-e29b-41d4-a716-446655440000", "name": "ACME", "industry": "Tech"}'::jsonb);

UPDATE test_tv_company
SET data = jsonb_smart_patch(data, '{"name": "ACME Corp"}'::jsonb, 'scalar')
WHERE pk_company = 1;

SELECT data->'name' = '"ACME Corp"'::jsonb AS name_updated,
       data->'industry' = '"Tech"'::jsonb AS industry_preserved
FROM test_tv_company WHERE pk_company = 1;
-- Expected: name_updated = true, industry_preserved = true

-- Test 2: pg_tview pattern - nested object
CREATE TABLE test_tv_user (
    pk_user INT PRIMARY KEY,
    fk_company INT,
    data JSONB
);

INSERT INTO test_tv_user VALUES
    (1, 1, '{"id": "...", "name": "Alice", "company": {"id": "...", "name": "ACME", "city": "NYC"}}'::jsonb);

UPDATE test_tv_user
SET data = jsonb_smart_patch(
    data,
    '{"name": "ACME Corp"}'::jsonb,
    'nested_object',
    path => ARRAY['company']
)
WHERE pk_user = 1;

SELECT data->'company'->>'name' = 'ACME Corp' AS company_name_updated,
       data->'company'->>'city' = 'NYC' AS city_preserved,
       data->>'name' = 'Alice' AS user_name_preserved
FROM test_tv_user WHERE pk_user = 1;
-- Expected: all true

-- Test 3: pg_tview pattern - array update
CREATE TABLE test_tv_feed (
    pk_feed INT PRIMARY KEY,
    data JSONB
);

INSERT INTO test_tv_feed VALUES
    (1, '{"posts": [{"id": "1", "title": "Post 1"}, {"id": "2", "title": "Post 2"}]}'::jsonb);

UPDATE test_tv_feed
SET data = jsonb_smart_patch(
    data,
    '{"title": "Updated Post 1", "featured": true}'::jsonb,
    'array',
    path => ARRAY['posts'],
    array_match_key => 'id',
    match_value => '"1"'::jsonb
)
WHERE pk_feed = 1;

SELECT data->'posts'->0->>'title' = 'Updated Post 1' AS title_updated,
       data->'posts'->0->'featured' = 'true'::jsonb AS featured_added,
       data->'posts'->1->>'title' = 'Post 2' AS other_post_unchanged
FROM test_tv_feed WHERE pk_feed = 1;
-- Expected: all true

-- Cleanup
DROP TABLE test_tv_company, test_tv_user, test_tv_feed;
```

---

## 📊 Benchmark Plan

### Baseline (Existing Functions)
```sql
-- Measure existing function performance
\timing on

SELECT jsonb_merge_shallow(data, '{"name": "new"}'::jsonb)
FROM (SELECT '{"id": 1, "name": "old"}'::jsonb AS data) t;
-- Record time: ~X ms

SELECT jsonb_merge_at_path(data, '{"name": "new"}'::jsonb, ARRAY['user'])
FROM (SELECT '{"id": 1, "user": {"name": "old"}}'::jsonb AS data) t;
-- Record time: ~Y ms

SELECT jsonb_array_update_where(data, 'items', 'id', '1'::jsonb, '{"name": "new"}'::jsonb)
FROM (SELECT '{"items": [{"id": 1, "name": "old"}]}'::jsonb AS data) t;
-- Record time: ~Z ms
```

### Smart Patch Performance
```sql
-- Same operations via jsonb_smart_patch
SELECT jsonb_smart_patch(data, '{"name": "new"}'::jsonb, 'scalar')
FROM (SELECT '{"id": 1, "name": "old"}'::jsonb AS data) t;
-- Expected: ~X + 0.1ms (dispatch overhead)

SELECT jsonb_smart_patch(data, '{"name": "new"}'::jsonb, 'nested_object', path => ARRAY['user'])
FROM (SELECT '{"id": 1, "user": {"name": "old"}}'::jsonb AS data) t;
-- Expected: ~Y + 0.1ms

SELECT jsonb_smart_patch(data, '{"name": "new"}'::jsonb, 'array', path => ARRAY['items'], array_match_key => 'id', match_value => '1'::jsonb)
FROM (SELECT '{"items": [{"id": 1, "name": "old"}]}'::jsonb AS data) t;
-- Expected: ~Z + 0.1ms
```

**Success Criterion:** Dispatch overhead < 0.1ms (< 5% of typical update time)

---

## 📝 Step-by-Step Implementation

### Day 1: Setup & Core Implementation
1. **Create module file**
   ```bash
   touch src/smart_patch.rs
   ```

2. **Add module to lib.rs**
   ```rust
   mod smart_patch;
   pub use smart_patch::jsonb_smart_patch;
   ```

3. **Implement function skeleton**
   - Function signature with all parameters
   - Basic match statement for patch_type
   - Error handling for invalid patch_type

4. **Implement scalar case**
   - Call `jsonb_merge_shallow`
   - Test with simple example

### Day 2: Nested Object & Array Cases
1. **Implement nested_object case**
   - Parameter validation (path required)
   - Call `jsonb_merge_at_path`
   - Test with nested example

2. **Implement array case**
   - Parameter validation (path, match_key, match_value required)
   - Extract array_path from path parameter
   - Call `jsonb_array_update_where`
   - Test with array example

### Day 3: Error Handling & Edge Cases
1. **Add parameter validation**
   - Check for missing required parameters
   - Clear error messages with parameter names

2. **Test edge cases**
   - Empty paths
   - NULL source
   - Non-existent paths
   - Invalid JSONB structures

### Day 4: Unit Tests
1. **Write comprehensive unit tests**
   - Test all three patch types
   - Test all error conditions
   - Test edge cases

2. **Run test suite**
   ```bash
   cargo pgrx test --release
   ```

3. **Fix any failing tests**

### Day 5: Integration Tests & Documentation
1. **Write integration tests** (SQL file)
   - Simulate pg_tview patterns
   - Test realistic scenarios

2. **Write documentation**
   - Doc comments with examples
   - Update README.md
   - Add to CHANGELOG.md

3. **Run benchmarks**
   - Measure dispatch overhead
   - Document results

4. **Code review** (self-review)
   - Check code style
   - Verify error messages
   - Ensure tests cover all branches

---

## 🚨 Potential Issues & Solutions

### Issue 1: Optional Parameters in pgrx
**Problem:** pgrx's handling of optional parameters with defaults can be tricky

**Solution:** Use `Option<default!(Type, "NULL")>` pattern consistently

**Example:**
```rust
path: Option<default!(pgrx::Array<&str>, "NULL")>
```

### Issue 2: Extracting First Element of Array
**Problem:** Getting first element of `pgrx::Array` requires iterator

**Solution:**
```rust
let array_path = path_array
    .iter()
    .next()
    .flatten()
    .expect("path must have at least one element");
```

### Issue 3: Error Messages Not Showing Parameter Name
**Problem:** Generic errors make debugging hard

**Solution:** Include parameter name in error message:
```rust
error!("path parameter is required for patch_type='nested_object'")
```

---

## ✅ Phase Completion Checklist

### Implementation
- [x] Functions implemented in `src/lib.rs`
- [x] `jsonb_smart_patch_scalar()` working
- [x] `jsonb_smart_patch_nested()` working
- [x] `jsonb_smart_patch_array()` working
- [x] All functions use existing primitives (zero additional complexity)

### Testing
- [x] SQL integration tests written (12 comprehensive tests)
- [x] All patch types tested
- [x] Error cases covered
- [x] All tests passing (100% success rate)
- [x] Edge cases covered (no match, string/int IDs, multiple fields)
- [x] pg_tview integration patterns validated

### Performance
- [x] Zero dispatch overhead (functions are thin wrappers)
- [x] Performance identical to underlying functions
- [x] No regressions in existing functions

### Documentation
- [x] Doc comments complete (with SQL examples)
- [x] pg_tview usage patterns documented
- [ ] README.md updated (pending)
- [ ] CHANGELOG.md updated (pending)
- [x] Integration examples provided in doc comments

### Quality
- [x] Code follows existing style
- [x] No compiler warnings
- [x] No clippy warnings
- [x] Self-review completed
- [x] Workaround implemented for pgrx SQL generation issue

---

**Next Phase:** [Phase 2: Array CRUD Operations](phase-2-array-crud.md)
