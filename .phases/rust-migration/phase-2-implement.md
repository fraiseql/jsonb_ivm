# Phase 2: Implement jsonb_merge_shallow in Rust

**Objective**: Translate `jsonb_merge_shallow` from C to Rust using pgrx, maintaining identical behavior

**Status**: GREEN (Implementation)

**Prerequisites**: Phase 1 complete (Rust toolchain and pgrx configured)

---

## 🎯 Scope

Implement the core merge function in Rust with:
- Exact same SQL API signature
- Identical behavior to C version
- Memory safety guaranteed by Rust
- Leverage pgrx's `JsonB` type wrappers
- Proper error handling with PostgreSQL integration

---

## 🔍 C Implementation Reference

**From old `jsonb_ivm.c`:**
```c
PG_FUNCTION_INFO_V1(jsonb_merge_shallow);
Datum
jsonb_merge_shallow(PG_FUNCTION_ARGS)
{
    Jsonb *target;
    Jsonb *source;

    /* NULL handling */
    if (PG_ARGISNULL(0) || PG_ARGISNULL(1))
        PG_RETURN_NULL();

    /* Extract arguments */
    target = PG_GETARG_JSONB_P(0);
    source = PG_GETARG_JSONB_P(1);

    /* Validate both are objects */
    if (!JB_ROOT_IS_OBJECT(target))
        ereport(ERROR, ...);
    if (!JB_ROOT_IS_OBJECT(source))
        ereport(ERROR, ...);

    /* Delegate to jsonb_concat */
    PG_RETURN_JSONB_P(jsonb_concat(target, source));
}
```

**Key behaviors:**
1. Returns `NULL` if either argument is `NULL`
2. Errors if either argument is not a JSONB object (rejects arrays, scalars)
3. Delegates to PostgreSQL's `jsonb_concat` (||) operator
4. Source keys overwrite target keys on conflicts

---

## 🦀 Rust Implementation

### Step 1: Update `src/lib.rs` with Full Implementation

Replace `src/lib.rs` contents:

```rust
// jsonb_ivm - Incremental JSONB View Maintenance Extension
//
// High-performance PostgreSQL extension for intelligent partial updates
// of JSONB materialized views in CQRS architectures.
//
// Copyright (c) 2025, Lionel Hamayon
// Licensed under the PostgreSQL License

use pgrx::prelude::*;
use serde_json::{Map, Value};

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
#[pg_schema]
mod tests {
    use pgrx::prelude::*;
    use pgrx::JsonB;
    use serde_json::json;

    #[pg_test]
    fn test_basic_merge() {
        let target = JsonB(json!({"a": 1, "b": 2}));
        let source = JsonB(json!({"c": 3}));

        let result = crate::jsonb_merge_shallow(Some(target), Some(source))
            .expect("merge should succeed");

        assert_eq!(result.0, json!({"a": 1, "b": 2, "c": 3}));
    }

    #[pg_test]
    fn test_overlapping_keys() {
        let target = JsonB(json!({"a": 1, "b": 2}));
        let source = JsonB(json!({"b": 99, "c": 3}));

        let result = crate::jsonb_merge_shallow(Some(target), Some(source))
            .expect("merge should succeed");

        // Source value (99) should overwrite target value (2)
        assert_eq!(result.0, json!({"a": 1, "b": 99, "c": 3}));
    }

    #[pg_test]
    fn test_empty_source() {
        let target = JsonB(json!({"a": 1}));
        let source = JsonB(json!({}));

        let result = crate::jsonb_merge_shallow(Some(target), Some(source))
            .expect("merge should succeed");

        assert_eq!(result.0, json!({"a": 1}));
    }

    #[pg_test]
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

    #[pg_test]
    #[should_panic(expected = "target argument must be a JSONB object")]
    fn test_array_target_errors() {
        let target = JsonB(json!([1, 2, 3]));
        let source = JsonB(json!({"a": 1}));

        // This should error
        let _ = crate::jsonb_merge_shallow(Some(target), Some(source));
    }

    #[pg_test]
    #[should_panic(expected = "source argument must be a JSONB object")]
    fn test_array_source_errors() {
        let target = JsonB(json!({"a": 1}));
        let source = JsonB(json!([1, 2, 3]));

        // This should error
        let _ = crate::jsonb_merge_shallow(Some(target), Some(source));
    }
}
```

---

## 🧪 Verification Steps

### Step 1: Build the Extension

```bash
cargo build --release
```

**Expected:**
```
   Compiling jsonb_ivm v0.1.0
    Finished release [optimized] target(s) in 12.3s
```

**No warnings allowed!** Fix any warnings before proceeding.

---

### Step 2: Run Rust Unit Tests

```bash
# Run pgrx's built-in tests
cargo pgrx test pg17

# This will:
# 1. Start a temporary PostgreSQL instance
# 2. Install the extension
# 3. Run all #[pg_test] functions
# 4. Report results
```

**Expected Output:**
```
running 6 tests
test tests::test_basic_merge ... ok
test tests::test_overlapping_keys ... ok
test tests::test_empty_source ... ok
test tests::test_null_handling ... ok
test tests::test_array_target_errors ... ok
test tests::test_array_source_errors ... ok

test result: ok. 6 passed; 0 failed
```

---

### Step 3: Generate SQL Schema

```bash
# Regenerate SQL file from Rust code
cargo pgrx schema
```

This creates `sql/jsonb_ivm--0.1.0.sql` with:
```sql
CREATE FUNCTION jsonb_merge_shallow(
    target jsonb,
    source jsonb
) RETURNS jsonb
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_merge_shallow_wrapper'
IMMUTABLE PARALLEL SAFE STRICT;

COMMENT ON FUNCTION jsonb_merge_shallow(jsonb, jsonb) IS
'Merge top-level keys from source JSONB into target JSONB...';
```

---

### Step 4: Test in Live PostgreSQL

```bash
# Start development PostgreSQL with extension loaded
cargo pgrx run pg17

# This opens psql with extension installed
```

**In psql:**
```sql
-- Verify extension is loaded
\dx jsonb_ivm

-- Test basic functionality
SELECT jsonb_merge_shallow(
    '{"a": 1, "b": 2}'::jsonb,
    '{"b": 99, "c": 3}'::jsonb
);
-- Expected: {"a": 1, "b": 99, "c": 3}

-- Test NULL handling
SELECT jsonb_merge_shallow(NULL, '{"a": 1}'::jsonb);
-- Expected: NULL

-- Test error handling (should error)
SELECT jsonb_merge_shallow('[1,2,3]'::jsonb, '{"a": 1}'::jsonb);
-- Expected: ERROR: target argument must be a JSONB object, got: array
```

---

## 🔍 Code Quality Checks

### Step 1: Format Code

```bash
# Format Rust code with rustfmt
cargo fmt

# Verify no changes needed
cargo fmt -- --check
```

---

### Step 2: Run Clippy (Rust Linter)

```bash
# Run clippy for best practices and common mistakes
cargo clippy --all-targets --all-features -- -D warnings

# This should pass with zero warnings
```

**Fix any clippy warnings before proceeding!**

---

### Step 3: Check Documentation

```bash
# Build documentation
cargo doc --no-deps --open

# Verify that:
# - jsonb_merge_shallow is documented
# - All parameters explained
# - Examples are present
```

---

## 📊 Performance Validation

While we don't have formal benchmarks yet, verify basic performance:

```sql
-- In cargo pgrx run pg17 psql session

-- Small merge (should be instant)
\timing on
SELECT jsonb_merge_shallow(
    '{"a": 1, "b": 2, "c": 3}'::jsonb,
    '{"d": 4, "e": 5}'::jsonb
);
-- Expected: < 1ms

-- Large merge (100 keys)
WITH target AS (
    SELECT jsonb_object_agg('key' || i, i) AS obj
    FROM generate_series(1, 100) i
),
source AS (
    SELECT jsonb_object_agg('key' || i, i * 10) AS obj
    FROM generate_series(51, 150) i
)
SELECT jsonb_merge_shallow(target.obj, source.obj)
FROM target, source;
-- Expected: < 5ms
```

---

## ✅ Acceptance Criteria

**This phase is complete when:**

- [ ] `src/lib.rs` contains complete `jsonb_merge_shallow` implementation
- [ ] Rust code compiles with zero warnings (`cargo build --release`)
- [ ] All Rust unit tests pass (`cargo pgrx test pg17`)
- [ ] SQL schema generated successfully (`cargo pgrx schema`)
- [ ] Manual testing in psql succeeds (basic, NULL, error cases)
- [ ] `cargo fmt -- --check` passes (code properly formatted)
- [ ] `cargo clippy` passes with `-D warnings` (zero linter warnings)
- [ ] Documentation is complete and accurate
- [ ] Performance is acceptable (< 5ms for 150-key merge)

---

## 🚫 DO NOT

- ❌ Add additional functions yet (only `jsonb_merge_shallow` in v0.1.0-alpha1)
- ❌ Optimize prematurely (current impl is sufficient)
- ❌ Update CI/CD yet (Phase 4)
- ❌ Commit to git yet (wait until all phases complete)
- ❌ Add complex examples (keep minimal for alpha1)

---

## 📝 Troubleshooting

### Issue: Compilation Errors

```bash
# Check pgrx version compatibility
cargo pgrx --version
# Should be 0.12.x

# Update dependencies
cargo update
```

### Issue: Tests Fail

```bash
# Get verbose output
cargo pgrx test pg17 -- --nocapture

# Check PostgreSQL logs
tail -f ~/.pgrx/data-17/postgresql.log
```

### Issue: Schema Generation Fails

```bash
# Clean and regenerate
rm -rf sql/
cargo clean
cargo pgrx schema
```

---

## 🎓 Key Differences from C

| Aspect | C Implementation | Rust + pgrx Implementation |
|--------|------------------|---------------------------|
| **Memory Safety** | Manual management | Compiler-verified borrowing |
| **Null Handling** | `PG_ARGISNULL` macros | `Option<T>` type |
| **Error Handling** | `ereport(ERROR, ...)` | `error!` macro (same effect) |
| **Type Safety** | Runtime checks | Compile-time checks |
| **Build System** | PGXS Makefile | Cargo + pgrx |
| **Testing** | External pg_regress | Integrated `#[pg_test]` |
| **Documentation** | Manual comments | Rustdoc (/// comments) |

---

## ⏭️ Next Phase

**Phase 3**: SQL Integration Tests
- Run original 12 SQL tests from `test/sql/01_merge_shallow.sql`
- Verify 100% compatibility with C version
- Ensure all expected outputs match

---

**Progress**: Phase 2 of 4 in Rust migration. Implementation complete, ready for comprehensive SQL testing.
