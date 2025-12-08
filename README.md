# jsonb_ivm - Incremental JSONB View Maintenance

[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-13%2B-blue.svg)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/License-PostgreSQL-blue.svg)](LICENSE)

**High-performance PostgreSQL extension for intelligent partial updates of JSONB in CQRS architectures.**

> ⚠️ **Alpha Release**: This is v0.3.0. pg_tview integration helpers added (complete CRUD support). Not recommended for production use yet.

---

## 🚀 Quick Start

### Installation

**From source (requires Rust + pgrx):**

```bash
# Install Rust if not already installed
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-pgrx
cargo install --locked cargo-pgrx

# Initialize pgrx (one-time setup)
cargo pgrx init

# Clone and build
git clone https://github.com/fraiseql/jsonb_ivm.git
cd jsonb_ivm
cargo pgrx install --release

# Create extension in PostgreSQL
psql -d your_database -c "CREATE EXTENSION jsonb_ivm;"
```

### Basic Usage

```sql
-- Update single element in JSONB array (2.66× faster than native SQL)
SELECT jsonb_array_update_where(
    '{"dns_servers": [{"id": 42, "ip": "1.1.1.1"}, {"id": 43, "ip": "2.2.2.2"}]}'::jsonb,
    'dns_servers',  -- array path
    'id',           -- match key
    '42'::jsonb,    -- match value
    '{"ip": "8.8.8.8"}'::jsonb  -- updates to apply
);
-- → {"dns_servers": [{"id": 42, "ip": "8.8.8.8"}, {"id": 43, "ip": "2.2.2.2"}]}

-- Merge JSONB at specific path
SELECT jsonb_merge_at_path(
    '{"config": {"name": "old", "ttl": 300}}'::jsonb,
    '{"name": "new"}'::jsonb,
    ARRAY['config']
);
-- → {"config": {"name": "new", "ttl": 300}}

-- Shallow merge
SELECT jsonb_merge_shallow(
    '{"a": 1, "b": 2}'::jsonb,
    '{"b": 99, "c": 3}'::jsonb
);
-- → {"a": 1, "b": 99, "c": 3}
```

---

## 📦 Features

### v0.3.0 (pg_tview Integration Helpers ⚡)

- ✅ **`jsonb_smart_patch_*()`** - Intelligent dispatchers (simplifies pg_tview by 60%)
- ✅ **`jsonb_array_delete_where()`** - Surgical array deletion (3-5× faster than re-aggregation)
- ✅ **`jsonb_array_insert_where()`** - Ordered array insertion (3-5× faster, NEW!)
- ✅ **`jsonb_deep_merge()`** - Recursive deep merge (preserves nested fields)
- ✅ **`jsonb_extract_id()` / `jsonb_array_contains_id()`** - Helper functions for pg_tview

**Impact on pg_tview:**
- Complete JSONB array CRUD support (INSERT/DELETE now available)
- 40-60% code reduction in refresh logic
- +10-20% cascade throughput improvement

### v0.2.0 (Performance Optimizations ⚡)

- ✅ **SIMD optimizations** - 6× faster for large arrays (1000+ elements)
- ✅ **`jsonb_array_update_where_batch()`** - Batch updates (3-5× faster)
- ✅ **`jsonb_array_update_multi_row()`** - Multi-row updates (4× faster for 100 rows)
- ✅ **Throughput improvement**: 167 → 357 ops/sec (+114%)
- ✅ **Cascade operations**: 2.4× faster vs v0.1.0

### v0.1.0 (POC Complete ✅)

- ✅ **`jsonb_array_update_where()`** - Surgical array element updates (2.66× faster)
- ✅ **`jsonb_merge_at_path()`** - Merge JSONB at nested paths
- ✅ **`jsonb_merge_shallow()`** - Shallow JSONB merge
- ✅ **PostgreSQL 17** compatible (pgrx 0.12.8)
- ✅ **Performance validated**: 1.45× to 2.66× faster than native SQL
- ✅ **Comprehensive benchmarks** included

### Performance Highlights

| Operation | Native SQL | Rust Extension | Speedup |
|-----------|-----------|----------------|---------|
| Single array update | 1.91 ms | **0.72 ms** | **2.66×** |
| Array-heavy cascade | 22.14 ms | **10.67 ms** | **2.08×** |
| 100 cascades (stress) | 870 ms | **600 ms** | **1.45×** |

See [benchmarks](docs/implementation/BENCHMARK_RESULTS.md) for details.

---

## 📖 API Reference

### v0.3.0 Functions

#### `jsonb_smart_patch_scalar(target, source)` ⭐ NEW

Intelligent shallow merge for top-level object updates.

**Example:**
```sql
UPDATE tv_company
SET data = jsonb_smart_patch_scalar(data, '{"name": "ACME Corp"}'::jsonb)
WHERE pk = 1;
```

#### `jsonb_smart_patch_nested(target, source, path)` ⭐ NEW

Merge JSONB at a nested path within the document.

**Example:**
```sql
UPDATE tv_user
SET data = jsonb_smart_patch_nested(
    data,
    '{"name": "ACME Corp"}'::jsonb,
    ARRAY['company']
)
WHERE fk_company = 1;
```

#### `jsonb_smart_patch_array(target, source, array_path, match_key, match_value)` ⭐ NEW

Update a specific element within a JSONB array.

**Example:**
```sql
UPDATE tv_feed
SET data = jsonb_smart_patch_array(
    data,
    '{"title": "Updated"}'::jsonb,
    'posts',
    'id',
    '"abc-123"'::jsonb
)
WHERE pk = 1;
```

#### `jsonb_array_delete_where(target, array_path, match_key, match_value)` ⭐ NEW

Surgically delete an element from a JSONB array.

**Performance:** 3-5× faster than re-aggregation.

**Example:**
```sql
UPDATE tv_feed
SET data = jsonb_array_delete_where(
    data,
    'posts',
    'id',
    '"post-to-delete"'::jsonb
)
WHERE pk = 1;
```

#### `jsonb_array_insert_where(target, array_path, new_element, sort_key, sort_order)` ⭐ NEW

Insert an element into a JSONB array with optional sorting.

**Performance:** 3-5× faster than re-aggregation.

**Example:**
```sql
UPDATE tv_feed
SET data = jsonb_array_insert_where(
    data,
    'posts',
    '{"id": "new-post", "title": "New Post", "created_at": "2025-12-08"}'::jsonb,
    'created_at',  -- sort by this key
    'DESC'         -- descending order
)
WHERE pk = 1;
```

#### `jsonb_deep_merge(target, source)` ⭐ NEW

Recursively merge nested JSONB objects, preserving fields not present in source.

**Example:**
```sql
SELECT jsonb_deep_merge(
    '{"a": 1, "b": {"c": 2, "d": 3}}'::jsonb,
    '{"b": {"d": 99}, "e": 4}'::jsonb
);
-- Result: {"a": 1, "b": {"c": 2, "d": 99}, "e": 4}
-- Note: "c" is preserved, unlike shallow merge which would lose it
```

#### `jsonb_extract_id(data, key)` ⭐ NEW

Safely extract an ID field from JSONB as text.

**Parameters:**
- `key` - defaults to `'id'`

**Example:**
```sql
SELECT jsonb_extract_id('{"id": "abc-123", "name": "test"}'::jsonb);
-- Result: "abc-123"
```

#### `jsonb_array_contains_id(data, array_path, id_key, id_value)` ⭐ NEW

Fast check if a JSONB array contains an element with a specific ID.

**Example:**
```sql
SELECT pk FROM tv_feed
WHERE jsonb_array_contains_id(
    data,
    'posts',
    'id',
    '"abc-123"'::jsonb
);
```

---

### v0.1.0 & v0.2.0 Functions

### `jsonb_array_update_where(target, array_path, match_key, match_value, updates)`

Updates a single element in a JSONB array by matching a key-value predicate.

**Parameters:**
- `target` (jsonb) - JSONB document containing the array
- `array_path` (text) - Path to the array (e.g., `'dns_servers'`)
- `match_key` (text) - Key to match on (e.g., `'id'`)
- `match_value` (jsonb) - Value to match (e.g., `'42'::jsonb`)
- `updates` (jsonb) - JSONB object to merge into matched element

**Returns:** Updated JSONB document

**Performance:** O(n) where n = array length. 2-3× faster than native SQL re-aggregation. With SIMD optimization (v0.2.0), up to 6× faster for large arrays (1000+ elements).

---

### `jsonb_array_update_where_batch(target, array_path, match_key, updates_array)` ⭐ NEW in v0.2.0

Batch update multiple elements in a JSONB array in a single pass.

**Parameters:**
- `target` (jsonb) - JSONB document containing the array
- `array_path` (text) - Path to the array
- `match_key` (text) - Key to match on
- `updates_array` (jsonb) - Array of `{match_value, updates}` objects

**Returns:** Updated JSONB document

**Example:**
```sql
SELECT jsonb_array_update_where_batch(
    '{"dns_servers": [{"id": 1}, {"id": 2}, {"id": 3}]}'::jsonb,
    'dns_servers',
    'id',
    '[
        {"match_value": 1, "updates": {"ip": "1.1.1.1"}},
        {"match_value": 2, "updates": {"ip": "2.2.2.2"}}
    ]'::jsonb
);
```

**Performance:** O(n+m) where n=array length, m=updates count. **3-5× faster** than m separate function calls.

---

### `jsonb_array_update_multi_row(targets, array_path, match_key, match_value, updates)` ⭐ NEW in v0.2.0

Update arrays across multiple JSONB documents in one call.

**Parameters:**
- `targets` (jsonb[]) - Array of JSONB documents
- `array_path` (text) - Path to array in each document
- `match_key` (text) - Key to match on
- `match_value` (jsonb) - Value to match
- `updates` (jsonb) - JSONB object to merge

**Returns:** Array of updated JSONB documents (same order as input)

**Example:**
```sql
-- Update 100 network configurations in one call
UPDATE tv_network_configuration
SET data = batch_result[ordinality]
FROM (
    SELECT unnest(
        jsonb_array_update_multi_row(
            array_agg(data ORDER BY id),
            'dns_servers',
            'id',
            '42'::jsonb,
            '{"ip": "8.8.8.8"}'::jsonb
        )
    ) WITH ORDINALITY AS batch_result
    FROM tv_network_configuration
    WHERE network_id = 17
) batch;
```

**Performance:** Amortizes FFI overhead. **~4× faster** for 100-row batches.

---

### `jsonb_merge_at_path(target, source, path)`

Merges a JSONB object at a specific nested path.

**Parameters:**
- `target` (jsonb) - Base JSONB document
- `source` (jsonb) - JSONB object to merge
- `path` (text[]) - Path array (e.g., `ARRAY['network', 'config']`)

**Returns:** Updated JSONB with source merged at path

**Performance:** O(depth) where depth = path length. Efficient for deep updates.

---

### `jsonb_merge_shallow(target, source)`

Merges top-level keys from source into target (shallow merge).

**Parameters:**
- `target` (jsonb) - Base JSONB object
- `source` (jsonb) - JSONB object whose keys will be merged

**Returns:** New JSONB object with merged keys

**Behavior:** Source keys overwrite target keys on conflict. Nested objects are replaced, not recursively merged.

---

## 🧪 Testing

```bash
# Run Rust test suite
cargo pgrx test --release

# Run performance benchmarks
psql -d postgres -f test/benchmark_array_update_where.sql
```

Test coverage:
- ✅ All 3 functions with edge cases
- ✅ NULL handling (strict attribute)
- ✅ Type validation
- ✅ Performance benchmarks vs native SQL

---

## 📊 Use Cases

### CQRS Incremental View Maintenance

**Scenario:** Update DNS server affecting 100 network configurations

**Before (Native SQL):**
```sql
-- Re-aggregate entire array (slow!)
UPDATE tv_network_configuration
SET data = jsonb_set(data, '{dns_servers}', (
    SELECT jsonb_agg(v.data ORDER BY m.priority)
    FROM bench_nc_dns_mapping m
    JOIN v_dns_server v ON v.id = m.dns_server_id
    WHERE m.network_configuration_id = tv_network_configuration.id
))
WHERE id IN (SELECT network_configuration_id FROM mappings WHERE dns_server_id = 42);
-- Time: ~22ms for 100 rows
```

**After (Rust Extension):**
```sql
-- Surgical update (fast!)
UPDATE tv_network_configuration
SET data = jsonb_array_update_where(
    data,
    'dns_servers',
    'id',
    '42'::jsonb,
    (SELECT data FROM v_dns_server WHERE id = 42)
)
WHERE id IN (SELECT network_configuration_id FROM mappings WHERE dns_server_id = 42);
-- Time: ~10.7ms for 100 rows (2.08× faster)
```

**Impact:** 46% throughput improvement (114 → 167 ops/sec)

---

## 📚 Documentation

- **[pg_tview Integration Examples](docs/PG_TVIEW_INTEGRATION_EXAMPLES.md)** - Real-world CRUD workflows (NEW v0.3.0)
- **[Implementation Details](docs/implementation/IMPLEMENTATION_SUCCESS.md)** - Technical implementation and verification
- **[Benchmark Results](docs/implementation/BENCHMARK_RESULTS.md)** - Complete performance analysis
- **[pgrx Integration Notes](docs/implementation/PGRX_INTEGRATION_ISSUE.md)** - SQL generation troubleshooting
- **[Development Guide](DEVELOPMENT.md)** - Building and testing
- **[Changelog](CHANGELOG.md)** - Version history

### Archived Documentation

- [Phase Plans](docs/archive/phases/) - Implementation phase history
- [POC Planning](docs/archive/) - Original POC documentation

---

## 🛠️ Requirements

- **PostgreSQL**: 17 (tested with pgrx 0.12.8)
- **Rust**: Stable toolchain
- **pgrx**: 0.12.8
- **OS**: Linux (tested), macOS (should work)

### Build Dependencies

**Debian/Ubuntu:**
```bash
sudo apt-get install postgresql-server-dev-17 build-essential libclang-dev
```

**Arch Linux:**
```bash
sudo pacman -S postgresql-libs base-devel clang
```

---

## 🤝 Contributing

This project is in alpha. Feedback and bug reports welcome!

**Found a bug?** Open an issue: https://github.com/fraiseql/jsonb_ivm/issues

---

## 📜 License

Licensed under the PostgreSQL License. See [LICENSE](LICENSE) for details.

---

## 👤 Author

**Lionel Hamayon** - [fraiseql](https://github.com/fraiseql)

---

## 🎯 Project Status

**v0.3.0 Status:** ✅ **pg_tview Integration Complete**

- 13 functions implemented (v0.1.0 + v0.2.0 + v0.3.0)
- Complete JSONB array CRUD support (CREATE, READ, UPDATE, DELETE)
- Performance validated (3-5× faster for INSERT/DELETE operations)
- Comprehensive benchmarks and pg_tview integration examples
- Ready for integration with pg_tview project

**Next Steps:**
- Integration with pg_tview (replace manual refresh logic)
- Additional PostgreSQL version support (13-16)
- Further SIMD optimizations
- Production readiness hardening

---

**Built with PostgreSQL ❤️ and Rust 🦀 | Alpha Quality | Performance Validated**
