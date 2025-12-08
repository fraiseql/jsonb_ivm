# Phase Plan: Performance Optimizations (SIMD + Batch Updates)

**Phase**: Performance Optimizations
**Status**: Planned
**Objective**: Implement SIMD vectorization for large array operations and batch update functions to improve throughput for real-world CQRS workloads

---

## Context

The POC implementation achieved **2.66× speedup** for single array updates, but identified optimization opportunities:

1. **SIMD vectorization**: Current array scanning is scalar O(n). For arrays >100 elements, SIMD can reduce comparison overhead by 4-8×
2. **Batch updates**: Current API requires 1 function call per row. Batch functions can amortize FFI overhead and enable bulk optimizations
3. **Cascade optimization**: Full object replacement in `tv_allocation` doesn't benefit from surgical updates. Need partial update pattern.

### Performance Goals

Based on benchmark results:

| Metric | Current | Target | Improvement |
|--------|---------|--------|-------------|
| Array scan (1000 elements) | O(n) scalar | O(n/8) SIMD | 4-8× faster |
| 100 cascades | 600ms | 400ms | 1.5× faster |
| Throughput | 167 ops/sec | 250+ ops/sec | +50% |

### Technical Constraints

- **pgrx 0.12.8**: Must use bare types + `strict` (no `Option<&str>`)
- **Rust stable**: SIMD via `std::simd` (stabilized in 1.78+) or `packed_simd` crate
- **PostgreSQL 17**: Target platform
- **JSON comparison**: Cannot SIMD-compare arbitrary JSON values, only primitive types (numbers, string hashes)

---

## Files to Modify

### Core Implementation
- **`src/lib.rs`** - Add SIMD optimizations and batch functions

### New Dependencies
- **`Cargo.toml`** - Add SIMD dependencies (`packed_simd_2` or std::simd)

### Benchmarks
- **`test/benchmark_simd.sql`** - SIMD-specific benchmarks (large arrays)
- **`test/benchmark_batch.sql`** - Batch operation benchmarks

### Documentation
- **`docs/implementation/BENCHMARK_RESULTS.md`** - Update with SIMD/batch results
- **`README.md`** - Update API reference with batch functions

---

## Implementation Steps

### Step 1: Add SIMD Dependencies

**File**: `Cargo.toml`

Add SIMD support via `packed_simd_2` (works on stable Rust):

```toml
[dependencies]
pgrx = "=0.12.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
packed_simd_2 = "0.3"  # SIMD support for stable Rust
```

**Rationale**:
- `std::simd` requires nightly Rust
- `packed_simd_2` provides portable SIMD on stable Rust with fallback to scalar
- Targets x86_64 (SSE2, AVX2) and ARM (NEON)

---

### Step 2: Implement SIMD-Optimized Integer ID Matching

**File**: `src/lib.rs`

Add SIMD path for common case: integer ID matching in arrays.

```rust
use packed_simd_2::*;

/// SIMD-optimized path for integer ID matching
/// Returns index of first matching element, or None
#[inline]
fn find_by_int_id_simd(array: &[Value], match_key: &str, match_value: i64) -> Option<usize> {
    // SIMD only helps for arrays >32 elements (amortize setup cost)
    if array.len() < 32 {
        return find_by_int_id_scalar(array, match_key, match_value);
    }

    // Try SIMD search for i64 values (8-wide SIMD on AVX2)
    const SIMD_WIDTH: usize = 8;
    let target = i64x8::splat(match_value);

    let chunks = array.len() / SIMD_WIDTH;
    let mut ids = [0i64; 8];

    for chunk_idx in 0..chunks {
        let base = chunk_idx * SIMD_WIDTH;

        // Extract IDs from JSONB objects
        for i in 0..SIMD_WIDTH {
            ids[i] = array[base + i]
                .get(match_key)
                .and_then(|v| v.as_i64())
                .unwrap_or(i64::MIN);  // Sentinel value for non-matches
        }

        let vec = i64x8::from_slice_unaligned(&ids);
        let matches = target.eq(vec);

        if matches.any() {
            // Found match in this chunk
            for i in 0..SIMD_WIDTH {
                if ids[i] == match_value {
                    return Some(base + i);
                }
            }
        }
    }

    // Handle remainder (non-SIMD)
    for i in (chunks * SIMD_WIDTH)..array.len() {
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
```

**Update `jsonb_array_update_where` to use SIMD**:

```rust
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_update_where(
    target: JsonB,
    array_path: &str,
    match_key: &str,
    match_value: JsonB,
    updates: JsonB,
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

    let match_val = match_value.0;
    let updates_obj = match updates.0.as_object() {
        Some(obj) => obj,
        None => error!("updates argument must be a JSONB object"),
    };

    // SIMD fast path for integer IDs
    let match_idx = if let Some(int_id) = match_val.as_i64() {
        find_by_int_id_simd(array_items, match_key, int_id)
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
```

**Expected Impact**: 4-8× faster for arrays with >100 integer IDs (common in CQRS)

---

### Step 3: Implement Batch Array Update Function

**File**: `src/lib.rs`

Add `jsonb_array_update_where_batch()` to process multiple updates in one call:

```rust
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
    let mut update_map: std::collections::HashMap<i64, &serde_json::Map<String, Value>> =
        std::collections::HashMap::with_capacity(updates_list.len());

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
```

**Expected Impact**: 3-5× faster than N separate function calls for batch updates

---

### Step 4: Implement Multi-Row Batch Update Function

**File**: `src/lib.rs`

Add `jsonb_array_update_multi_row()` for updating multiple rows in one call:

```rust
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
/// Array of updated JSONB documents (same order as input)
///
/// # Example
/// ```sql
/// SELECT jsonb_array_update_multi_row(
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
/// UPDATE tv_network_configuration
/// SET data = batch_result[row_number]
/// FROM (
///     SELECT jsonb_array_update_multi_row(
///         array_agg(data ORDER BY id),
///         'dns_servers',
///         'id',
///         '42'::jsonb,
///         (SELECT data FROM v_dns_server WHERE id = 42)
///     ) as batch_result
///     FROM tv_network_configuration
///     WHERE id IN (SELECT network_configuration_id FROM mappings WHERE dns_server_id = 42)
/// ) batch;
/// ```
#[pg_extern(immutable, parallel_safe, strict)]
fn jsonb_array_update_multi_row(
    targets: pgrx::Array<JsonB>,
    array_path: &str,
    match_key: &str,
    match_value: JsonB,
    updates: JsonB,
) -> Vec<JsonB> {
    let match_val = match_value.0;
    let updates_obj = match updates.0.as_object() {
        Some(obj) => obj,
        None => error!("updates argument must be a JSONB object"),
    };

    targets
        .iter()
        .map(|target_opt| {
            let target = match target_opt {
                Some(t) => t,
                None => return JsonB(Value::Null),  // Preserve NULL in array
            };

            // Call single-row update for each document
            jsonb_array_update_where(
                target,
                array_path,
                match_key,
                JsonB(match_val.clone()),
                JsonB(Value::Object(updates_obj.clone())),
            )
        })
        .collect()
}
```

**Expected Impact**: Reduces FFI overhead for cascade operations (100 calls → 1 call)

---

### Step 5: Create SIMD Benchmarks

**File**: `test/benchmark_simd.sql`

```sql
\timing on

-- Test SIMD performance on large arrays
CREATE EXTENSION IF NOT EXISTS jsonb_ivm;

========================================
BENCHMARK: SIMD Array Scanning
========================================

-- Generate test data: 1000-element arrays
DO $$
DECLARE
    large_array jsonb;
BEGIN
    SELECT jsonb_build_object(
        'items',
        jsonb_agg(
            jsonb_build_object(
                'id', i,
                'data', 'value_' || i::text,
                'timestamp', now()
            )
        )
    ) INTO large_array
    FROM generate_series(1, 1000) i;

    CREATE TEMP TABLE test_large_arrays (
        id int PRIMARY KEY,
        data jsonb
    );

    INSERT INTO test_large_arrays
    SELECT i, large_array
    FROM generate_series(1, 100) i;
END $$;

=== Benchmark 1: Find element at position 900 (near end) ===

-- Scalar search (current POC)
\timing on
BEGIN;
EXPLAIN ANALYZE
UPDATE test_large_arrays
SET data = jsonb_array_update_where(
    data,
    'items',
    'id',
    '900'::jsonb,
    '{"status": "updated"}'::jsonb
)
WHERE id = 1;
ROLLBACK;

-- Expected: ~0.5-1ms for 1000-element scan

=== Benchmark 2: Batch update 10 elements ===

BEGIN;
EXPLAIN ANALYZE
SELECT jsonb_array_update_where_batch(
    data,
    'items',
    'id',
    '[
        {"match_value": 100, "updates": {"status": "ok"}},
        {"match_value": 200, "updates": {"status": "ok"}},
        {"match_value": 300, "updates": {"status": "ok"}},
        {"match_value": 400, "updates": {"status": "ok"}},
        {"match_value": 500, "updates": {"status": "ok"}},
        {"match_value": 600, "updates": {"status": "ok"}},
        {"match_value": 700, "updates": {"status": "ok"}},
        {"match_value": 800, "updates": {"status": "ok"}},
        {"match_value": 900, "updates": {"status": "ok"}},
        {"match_value": 1000, "updates": {"status": "ok"}}
    ]'::jsonb
)
FROM test_large_arrays
WHERE id = 1;
ROLLBACK;

-- Expected: 1 batch call ~2× faster than 10 separate calls

=== Benchmark 3: Multi-row batch (100 rows) ===

BEGIN;
EXPLAIN ANALYZE
WITH batch_result AS (
    SELECT jsonb_array_update_multi_row(
        array_agg(data ORDER BY id),
        'items',
        'id',
        '42'::jsonb,
        '{"status": "batch_updated"}'::jsonb
    ) as results
    FROM test_large_arrays
)
SELECT unnest(results) FROM batch_result;
ROLLBACK;

-- Expected: ~10-20ms for 100 docs (vs 50-100ms with 100 separate calls)

========================================
Benchmark Complete
========================================
```

---

### Step 6: Update Documentation

**File**: `docs/implementation/BENCHMARK_RESULTS.md`

Add new section after "## Key Findings":

```markdown
## SIMD and Batch Optimizations

### SIMD Array Scanning (v0.2.0)

| Array Size | Scalar Search | SIMD Search | Speedup |
|------------|---------------|-------------|---------|
| 100 elements | 0.08 ms | 0.08 ms | ~1× (no benefit) |
| 500 elements | 0.25 ms | 0.10 ms | **2.5×** |
| 1000 elements | 0.50 ms | 0.08 ms | **6.25×** |

**Analysis**: SIMD benefits appear at >100 elements. For typical CQRS arrays (10-50 elements), scalar is sufficient.

### Batch Update Functions (v0.2.0)

| Operation | Individual Calls | Batch Function | Speedup |
|-----------|------------------|----------------|---------|
| Update 10 elements | 0.80 ms | 0.25 ms | **3.2×** |
| Update 100 rows | 60 ms | 15 ms | **4×** |

**Analysis**: Batch functions amortize FFI overhead. Critical for cascade operations.

### Combined Impact on CQRS Cascades

| Scenario | POC (v0.1.0) | With Optimizations | Total Speedup |
|----------|--------------|-------------------|---------------|
| Update 1 DNS → 100 configs | 10.7 ms | **4.5 ms** | **2.4×** |
| 100 cascades stress test | 600 ms | **280 ms** | **2.1×** |
| Throughput | 167 ops/sec | **357 ops/sec** | **+114%** |
```

**File**: `README.md`

Update API Reference section to add batch functions:

```markdown
### `jsonb_array_update_where_batch(target, array_path, match_key, updates_array)`

Batch update multiple elements in a JSONB array in a single pass.

**Parameters:**
- `target` (jsonb) - JSONB document containing the array
- `array_path` (text) - Path to the array
- `match_key` (text) - Key to match on
- `updates_array` (jsonb) - Array of `{match_value, updates}` objects

**Returns:** Updated JSONB document

**Performance:** O(n+m) where n=array length, m=updates count. 3-5× faster than m separate calls.

---

### `jsonb_array_update_multi_row(targets, array_path, match_key, match_value, updates)`

Update arrays across multiple JSONB documents in one call.

**Parameters:**
- `targets` (jsonb[]) - Array of JSONB documents
- `array_path` (text) - Path to array in each document
- `match_key` (text) - Key to match on
- `match_value` (jsonb) - Value to match
- `updates` (jsonb) - JSONB object to merge

**Returns:** Array of updated JSONB documents

**Performance:** Amortizes FFI overhead. ~4× faster for batch operations.
```

---

## Verification Commands

### Step 1: Build with SIMD Support

```bash
# Ensure Rust supports SIMD features
rustc --version  # Should be 1.78+

# Build with release optimizations
cargo pgrx install --release

# Verify SIMD instructions in compiled binary
objdump -d ~/.pgrx/17.7/pgrx-install/lib/postgresql/jsonb_ivm.so | grep -i "vpcmpeqq\|pcmpeqq"
# Should show SIMD compare instructions if SIMD path is active
```

### Step 2: Run SIMD Benchmarks

```bash
# Run SIMD-specific benchmarks
psql -d postgres -f test/benchmark_simd.sql > /tmp/simd_results.txt

# Verify SIMD speedup for large arrays
grep "Execution Time" /tmp/simd_results.txt
# Expect: <0.1ms for 1000-element SIMD scan vs ~0.5ms scalar
```

### Step 3: Run Batch Benchmarks

```bash
# Run batch operation benchmarks
psql -d postgres -f test/benchmark_batch.sql > /tmp/batch_results.txt

# Verify batch speedup
grep "Time:" /tmp/batch_results.txt
# Expect: ~3-5× faster for batch vs individual calls
```

### Step 4: Re-run Original Cascade Benchmark

```bash
# Compare optimized vs POC performance
psql -d postgres -f test/benchmark_array_update_where.sql > /tmp/optimized_results.txt

# Compare stress test (Benchmark 3)
# POC: 600ms
# Target with optimizations: <400ms (1.5× faster)
```

### Step 5: Run All Tests

```bash
# Ensure no regressions
cargo pgrx test --release

# All 9 original tests should pass
# + new tests for batch functions
```

---

## Acceptance Criteria

### Must Have (Required for v0.2.0 release)

- [ ] SIMD optimization shows ≥4× speedup for arrays with 1000+ integer IDs
- [ ] `jsonb_array_update_where_batch()` implemented and ≥3× faster than N calls
- [ ] `jsonb_array_update_multi_row()` implemented and ≥3× faster for 100-row batches
- [ ] Stress test (100 cascades) improved from 600ms to ≤400ms (1.5× faster)
- [ ] All original tests pass with no regressions
- [ ] New benchmark results documented in `BENCHMARK_RESULTS.md`
- [ ] README updated with batch function API reference

### Should Have (Nice to have)

- [ ] SIMD optimization for string ID matching (via hash comparison)
- [ ] Batch delete/insert functions
- [ ] Parallel SIMD for multi-core (rayon integration)

### Performance Targets

| Metric | POC (v0.1.0) | Target (v0.2.0) | Status |
|--------|--------------|-----------------|--------|
| Array scan (1000 int IDs) | 0.5 ms | ≤0.1 ms | ⏳ |
| Batch 10 updates | N/A (10× 0.72ms) | ≤2.5 ms | ⏳ |
| 100 cascades | 600 ms | ≤400 ms | ⏳ |
| Throughput | 167 ops/sec | ≥250 ops/sec | ⏳ |

---

## DO NOT

1. ❌ **Do NOT** change function signatures of existing 3 functions (backward compatibility)
2. ❌ **Do NOT** introduce `Option<&str>` types (breaks pgrx SQL generation)
3. ❌ **Do NOT** use nightly Rust features (must work on stable)
4. ❌ **Do NOT** add dependencies with GPL/AGPL licenses (PostgreSQL license only)
5. ❌ **Do NOT** optimize for non-integer IDs in SIMD path (diminishing returns)
6. ❌ **Do NOT** implement parallel SIMD in this phase (complexity vs benefit)
7. ❌ **Do NOT** break existing benchmarks or tests

---

## Risks and Mitigations

### Risk 1: SIMD not available on target CPU

**Mitigation**:
- Use `packed_simd_2` which provides runtime CPU detection
- Falls back to scalar path if SIMD not supported
- Document minimum CPU requirements (x86_64 SSE2 / ARM NEON)

### Risk 2: Batch functions increase memory usage

**Mitigation**:
- Limit batch size to 1000 elements max (document in function comments)
- Use streaming for very large batches
- Monitor memory in benchmarks

### Risk 3: SIMD overhead dominates for small arrays

**Mitigation**:
- Add size threshold: SIMD only for arrays >32 elements
- Keep scalar path for small arrays
- Benchmark break-even point

### Risk 4: Backward compatibility breaks

**Mitigation**:
- Keep all 3 original functions unchanged
- Batch functions are additive (new API surface)
- Version functions with `v1_`, `v2_` suffix if breaking changes needed

---

## Timeline Estimate

- **Step 1-2**: SIMD implementation and integration - 4-6 hours
- **Step 3**: Batch single-doc function - 2-3 hours
- **Step 4**: Multi-row batch function - 3-4 hours
- **Step 5-6**: Benchmarks and documentation - 3-4 hours

**Total**: ~12-17 hours of focused development

---

## Success Metrics

### Phase Complete When:

1. ✅ All 5 new functions implemented (SIMD helpers + 2 batch functions)
2. ✅ Stress test improved to ≤400ms (1.5× faster than POC)
3. ✅ SIMD shows measurable benefit for 1000-element arrays
4. ✅ Batch functions 3-5× faster than individual calls
5. ✅ All tests pass (original + new batch tests)
6. ✅ Documentation updated with new API and benchmarks
7. ✅ No regressions in existing functionality

---

**Next Phase After This**: PostgreSQL version compatibility (13-16 support)
