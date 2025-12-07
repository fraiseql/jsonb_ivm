# Phase 3: SQL Integration Testing

**Objective**: Verify 100% compatibility with original C implementation using comprehensive SQL test suite

**Status**: ✅ COMPLETE (2025-12-07)

**Prerequisites**: Phase 2 complete (Rust implementation working)

---

## 🎯 Scope

Run the complete SQL test suite and verify:
- All 12 original tests pass
- Output matches expected results exactly
- Error messages are helpful and accurate
- Performance is acceptable
- No regressions from C version

---

## 🧪 Test Strategy

### Existing Tests (from Phase 1 - C version)

We have 12 comprehensive tests in `test/sql/01_merge_shallow.sql`:

1. Basic merge
2. Overlapping keys (source overwrites)
3. Empty source
4. Empty target
5. Both empty
6. NULL target
7. NULL source
8. Nested objects (shallow replacement)
9. Different value types
10. Large object (150 keys)
11. Unicode support
12. Type validation (array errors)

**All must pass with identical output to `test/expected/01_merge_shallow.out`**

---

## 🛠️ Implementation Steps

### Step 1: Install Extension for Testing

```bash
# Install extension to system PostgreSQL
cargo pgrx install --release

# This installs to PostgreSQL's extension directory
```

**Expected Output:**
```
Building extension with features ``
  Finished release [optimized] target(s)
Installing extension
    Copying jsonb_ivm.so to /usr/lib/postgresql/17/lib
    Copying jsonb_ivm.control to /usr/share/postgresql/17/extension
    Copying jsonb_ivm--0.1.0.sql to /usr/share/postgresql/17/extension
```

---

### Step 2: Run pg_regress Tests

pgrx doesn't use pg_regress by default, so we need to adapt our tests:

```bash
# Create test runner script
cat > run_tests.sh << 'EOF'
#!/bin/bash
set -e

# Use pgrx test framework
cargo pgrx test pg17 --features pg_test

echo "✓ All Rust unit tests passed"

# Run SQL regression tests manually
echo "Running SQL integration tests..."

# Start test database
psql -d postgres -c "DROP DATABASE IF EXISTS test_jsonb_ivm;"
psql -d postgres -c "CREATE DATABASE test_jsonb_ivm;"

# Run test SQL
psql -d test_jsonb_ivm -f test/sql/01_merge_shallow.sql > test/results/01_merge_shallow.out 2>&1

# Compare with expected output
if diff -u test/expected/01_merge_shallow.out test/results/01_merge_shallow.out; then
    echo "✓ SQL integration tests passed"
else
    echo "✗ SQL integration tests failed - see diff above"
    exit 1
fi

# Cleanup
psql -d postgres -c "DROP DATABASE test_jsonb_ivm;"
EOF

chmod +x run_tests.sh
```

---

### Step 3: Update SQL Tests for pgrx

The original C tests assume extension is already loaded. With pgrx, we may need small adjustments:

**Check if `test/sql/01_merge_shallow.sql` works as-is:**

```bash
# Create results directory
mkdir -p test/results

# Run a single test manually
psql -d postgres << 'EOF'
DROP DATABASE IF EXISTS test_jsonb_ivm;
CREATE DATABASE test_jsonb_ivm;
\c test_jsonb_ivm
CREATE EXTENSION jsonb_ivm;

-- Test 1: Basic merge
SELECT jsonb_merge_shallow(
    '{"a": 1, "b": 2}'::jsonb,
    '{"c": 3}'::jsonb
);
EOF
```

**Expected:**
```
 jsonb_merge_shallow
---------------------
 {"a": 1, "b": 2, "c": 3}
```

If this works, the test file is compatible!

---

### Step 4: Run Full Test Suite

```bash
# Run all tests
./run_tests.sh
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
✓ All Rust unit tests passed

Running SQL integration tests...
✓ SQL integration tests passed
```

---

### Step 5: Analyze Test Coverage

```bash
# Install coverage tool
cargo install cargo-tarpaulin

# Run coverage analysis
cargo tarpaulin --out Html --output-dir coverage

# Open coverage/index.html in browser
```

**Target: 100% line coverage for `src/lib.rs`**

---

## 🔍 Test Case Verification

For each of the 12 tests, manually verify:

### Test 1-5: Basic Functionality
```sql
-- Should all pass, verify output format matches exactly
```

### Test 6-7: NULL Handling
```sql
-- Verify NULL is returned (not empty object)
SELECT jsonb_merge_shallow(NULL::jsonb, '{"a": 1}'::jsonb) IS NULL;
-- Expected: t (true)
```

### Test 8: Shallow Merge Behavior
```sql
-- Nested objects should be REPLACED, not merged
SELECT jsonb_merge_shallow(
    '{"a": {"x": 1, "y": 2}, "b": 3}'::jsonb,
    '{"a": {"z": 3}}'::jsonb
);
-- Expected: {"a": {"z": 3}, "b": 3}
-- NOT: {"a": {"x": 1, "y": 2, "z": 3}, "b": 3}  ← WRONG
```

### Test 9: Type Diversity
```sql
-- Verify strings, numbers, booleans, arrays, nested objects all work
SELECT jsonb_merge_shallow(
    '{"a": 1, "b": "text", "c": true}'::jsonb,
    '{"d": [1,2,3], "e": {"nested": "object"}}'::jsonb
);
-- All types preserved correctly
```

### Test 10: Large Objects
```sql
-- Verify 150 keys merge correctly (100 from target, 100 from source, 50 overlap)
-- Expected: 150 total keys
```

### Test 11: Unicode/Emoji
```sql
-- Verify international characters preserved
SELECT jsonb_merge_shallow(
    '{"名前": "太郎"}'::jsonb,
    '{"ville": "Montréal", "emoji": "🚀"}'::jsonb
);
-- All Unicode should be preserved correctly
```

### Test 12: Error Handling
```sql
-- Verify helpful error message
SELECT jsonb_merge_shallow('[1,2,3]'::jsonb, '{"a": 1}'::jsonb);
-- Expected error: "target argument must be a JSONB object, got: array"
```

---

## 🐛 Debugging Test Failures

### Issue: Output Format Mismatch

```bash
# Check exact diff
diff -u test/expected/01_merge_shallow.out test/results/01_merge_shallow.out

# Common issues:
# - Extra whitespace
# - Different key ordering (JSONB may reorder)
# - Different error message format
```

**Solution for key ordering:**
```rust
// In Rust, serde_json preserves insertion order
// But PostgreSQL's jsonb_out may reorder keys
// This is OK - test should accept any valid JSON representation
```

### Issue: Error Messages Don't Match

```rust
// Update error messages in src/lib.rs to match C version exactly
error!("target argument must be a JSONB object, got: {}", value_type_name(&target_value));
```

### Issue: Performance Regression

```bash
# Benchmark specific test
\timing on
SELECT jsonb_merge_shallow(...);
\timing off

# If slower than C version, profile:
cargo build --release --profile profiling
perf record -g target/release/...
```

---

## 📊 Benchmark Validation

Create a simple benchmark to compare with C baseline:

```sql
-- Benchmark script: test/benchmark_simple.sql
\timing on

-- Small objects (10 keys)
SELECT count(*) FROM (
    SELECT jsonb_merge_shallow(
        jsonb_build_object('a', i, 'b', i+1, 'c', i+2),
        jsonb_build_object('d', i*10, 'e', i*20)
    )
    FROM generate_series(1, 10000) i
) sub;

-- Medium objects (50 keys)
WITH target AS (
    SELECT jsonb_object_agg('key' || i, i) AS obj
    FROM generate_series(1, 50) i
),
source AS (
    SELECT jsonb_object_agg('key' || i, i * 10) AS obj
    FROM generate_series(26, 75) i
)
SELECT count(*) FROM (
    SELECT jsonb_merge_shallow(target.obj, source.obj)
    FROM target, source, generate_series(1, 1000)
) sub;

-- Large objects (150 keys)
WITH target AS (
    SELECT jsonb_object_agg('key' || i, i) AS obj
    FROM generate_series(1, 100) i
),
source AS (
    SELECT jsonb_object_agg('key' || i, i * 10) AS obj
    FROM generate_series(51, 150) i
)
SELECT count(*) FROM (
    SELECT jsonb_merge_shallow(target.obj, source.obj)
    FROM target, source, generate_series(1, 100)
) sub;

\timing off
```

**Run benchmark:**
```bash
psql -d test_jsonb_ivm -f test/benchmark_simple.sql
```

**Expected Performance:**
- Small (10 keys): < 50ms for 10k merges
- Medium (50 keys): < 500ms for 1k merges
- Large (150 keys): < 200ms for 100 merges

**Compare with C version** (if you have old benchmarks from backup).

---

## ✅ Acceptance Criteria

**This phase is complete when:**

- [ ] All 12 SQL tests pass
- [ ] Output matches expected results exactly (or is valid equivalent)
- [ ] Error messages are clear and helpful
- [ ] NULL handling works correctly
- [ ] Type validation errors trigger appropriately
- [ ] Unicode/emoji support verified
- [ ] Performance is acceptable (within 2x of C baseline)
- [ ] Test coverage is 100% for core logic
- [ ] `./run_tests.sh` succeeds consistently
- [ ] No memory leaks (run under valgrind if paranoid)

---

## 🚫 DO NOT

- ❌ Skip failing tests (fix them!)
- ❌ Modify expected output to match incorrect results
- ❌ Add "TODO" markers for broken functionality
- ❌ Commit broken tests
- ❌ Optimize prematurely if performance is acceptable

---

## 📝 Test Failure Checklist

If a test fails:

1. **Reproduce manually** in psql
2. **Check error messages** - are they helpful?
3. **Verify Rust logic** - does it match C version?
4. **Check pgrx version** - is it latest stable?
5. **Review PostgreSQL logs** - any warnings?
6. **Compare with C version** - run same test on C impl from backup
7. **File bug** if pgrx issue (unlikely)
8. **Fix implementation** in Phase 2, then re-test

---

## 🎓 Quality Standards

### Test Pyramid for v0.1.0-alpha1

```
        /\
       /  \      1 SQL Integration Test Suite (12 tests)
      /    \
     /______\    6 Rust Unit Tests (#[pg_test])
    /        \
   /__________\  Compiler checks (type safety, borrow checker)
```

**All layers must pass for phase completion.**

---

## ⏭️ Next Phase

**Phase 4**: CI/CD Integration
- Update GitHub Actions for Rust/pgrx
- Multi-version PostgreSQL testing
- Automated clippy/rustfmt checks
- Release automation

---

**Progress**: Phase 3 of 4 in Rust migration. Quality validation complete, ready for CI/CD automation.
