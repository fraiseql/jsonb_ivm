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

### Impact on pg_tview

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

### `jsonb_smart_patch_scalar(target, source)` ⭐ NEW

Intelligent shallow merge for top-level object updates.

**Example:**

```sql
UPDATE tv_company
SET data = jsonb_smart_patch_scalar(data, '{"name": "ACME Corp"}'::jsonb)
WHERE pk_company = 1;
```

### `jsonb_smart_patch_nested(target, source, path)` ⭐ NEW

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

### `jsonb_smart_patch_array(target, source, array_path, match_key, match_value)` ⭐ NEW

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
WHERE pk_feed = 1;
```

### `jsonb_array_delete_where(target, array_path, match_key, match_value)` ⭐ NEW

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
WHERE pk_feed = 1;
```

### `jsonb_array_insert_where(target, array_path, new_element, sort_key, sort_order)` ⭐ NEW

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
WHERE pk_feed = 1;
```

### `jsonb_deep_merge(target, source)` ⭐ NEW

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

### `jsonb_extract_id(data, key)` ⭐ NEW

Safely extract an ID field from JSONB as text.

**Parameters:**
- `key` - defaults to `'id'`

**Example:**

```sql
SELECT jsonb_extract_id('{"id": "abc-123", "name": "test"}'::jsonb);
-- Result: "abc-123"
```

### `jsonb_array_contains_id(data, array_path, id_key, id_value)` ⭐ NEW

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

## ⚠️ NULL Handling & Error Behavior

### NULL Parameter Handling

Most functions are marked `STRICT` (returns NULL if any parameter is NULL):

```sql
-- Returns NULL (not an error)
SELECT jsonb_smart_patch_scalar(NULL, '{"name": "test"}'::jsonb);  -- NULL
SELECT jsonb_smart_patch_scalar('{"a": 1}'::jsonb, NULL);          -- NULL
```

**Exception:** `jsonb_array_insert_where()` allows NULL for `sort_key` and `sort_order` parameters:

```sql
-- Valid: unsorted insertion
SELECT jsonb_array_insert_where(data, 'posts', new_post, NULL, NULL);
```

### Missing Paths/Keys

Functions return the **original JSONB unchanged** when paths/keys don't exist:

```sql
-- Path doesn't exist → returns original unchanged
SELECT jsonb_array_delete_where(
    '{"other": "data"}'::jsonb,
    'posts',  -- doesn't exist in document
    'id',
    '42'::jsonb
);
-- Result: {"other": "data"} (unchanged)

-- Match value not found → returns original unchanged
SELECT jsonb_smart_patch_array(
    '{"posts": [{"id": 1}]}'::jsonb,
    '{"title": "New"}'::jsonb,
    'posts',
    'id',
    '99'::jsonb  -- element with id=99 doesn't exist
);
-- Result: {"posts": [{"id": 1}]} (unchanged)
```

### Type Mismatches

Functions gracefully handle type mismatches by returning the original JSONB:

```sql
-- Array path points to non-array → returns original
SELECT jsonb_array_update_where(
    '{"posts": "not an array"}'::jsonb,
    'posts',
    'id',
    '1'::jsonb,
    '{}'::jsonb
);
-- Result: {"posts": "not an array"} (unchanged, no error)
```

### Best Practices

```sql
-- 1. Check if operation succeeded (compare result)
UPDATE tv_feed
SET data = jsonb_array_delete_where(data, 'posts', 'id', '42'::jsonb)
WHERE jsonb_array_contains_id(data, 'posts', 'id', '42'::jsonb);
-- Only updates rows that actually contain the element

-- 2. Handle NULL parameters explicitly
SELECT COALESCE(
    jsonb_smart_patch_scalar(data, updates),
    data  -- fallback if function returns NULL
) FROM tv_company;

-- 3. Validate JSONB structure before calling
SELECT
    CASE
        WHEN jsonb_typeof(data->'posts') = 'array'
        THEN jsonb_array_update_where(data, 'posts', 'id', id_val, updates)
        ELSE data  -- return unchanged if not an array
    END
FROM tv_feed;
```

**For more details**, see:
- [Troubleshooting Guide](docs/troubleshooting.md) - Common issues and solutions
- [Integration Examples](docs/pg-tview-integration-examples.md) - Error handling patterns

---

## 🧪 Testing

### Quick Start

```bash
# Install task runner (one-time)
cargo install just

# Run all tests
just test

# Development workflow
just check      # Fast: formatting + clippy
just fix        # Auto-fix issues
just dev        # Fix + build
```

### Manual Testing

**Rust Unit Tests** (requires pgrx):
```bash
cargo pgrx test pg17
```

**SQL Integration Tests**:
```bash
cargo pgrx install --release
psql -d test_jsonb_ivm -c "CREATE EXTENSION jsonb_ivm;"
psql -d test_jsonb_ivm -f test/sql/01_basic_operations.sql
```

### Why not `cargo test`?

pgrx extensions are PostgreSQL plugins, not standalone programs. They require PostgreSQL runtime symbols (PG_exception_stack, CurrentMemoryContext, etc.) which aren't available during standard Rust linking.

**Use `cargo pgrx test`** instead, which:
- Initializes a test PostgreSQL instance
- Loads the extension as a dynamic library
- Runs tests inside the PostgreSQL runtime (like production)

For CI, we use SQL integration tests across PostgreSQL 13-17.

### Test Coverage

- ✅ **30+ Rust unit tests**: All functions, edge cases, NULL handling, type validation
- ✅ **5 SQL test suites**: Production-like usage patterns
- ✅ **Multi-version**: PostgreSQL 13-17 (Ubuntu + macOS)
- ✅ **Performance benchmarks**: Validated 2-3× speedup vs native SQL

---

## 📊 Use Cases

### CQRS Incremental View Maintenance

### Scenario: Update DNS server affecting 100 network configurations

#### Before (Native SQL)

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

#### After (Rust Extension)

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

#### Impact: 46% throughput improvement (114 → 167 ops/sec)

---

## 📚 Documentation

- **[pg_tview Integration Examples](docs/pg-tview-integration-examples.md)** - Real-world CRUD workflows (NEW v0.3.0)
- **[Implementation Details](docs/implementation/implementation-success.md)** - Technical implementation and verification
- **[Benchmark Results](docs/implementation/benchmark-results.md)** - Complete performance analysis
- **[pgrx Integration Notes](docs/implementation/pgrx-integration-issue.md)** - SQL generation troubleshooting
- **[Development Guide](development.md)** - Building and testing
- **[Changelog](changelog.md)** - Version history

### Archived Documentation

- [Phase Plans](docs/archive/phases/) - Implementation phase history
- [POC Planning](docs/archive/) - Original POC documentation

---

## 🛠️ Requirements

### PostgreSQL Compatibility

| PostgreSQL Version | Status | Notes |
|--------------------|--------|-------|
| 17 | ✅ **Fully Tested** | Recommended version |
| 16 | ✅ **Supported** | Should work (built with `pg16` feature) |
| 15 | ✅ **Supported** | Should work (built with `pg15` feature) |
| 14 | ✅ **Supported** | Should work (built with `pg14` feature) |
| 13 | ✅ **Supported** | Should work (built with `pg13` feature) |
| 12 | ⚠️ **Experimental** | Untested, may work (built with `pg12` feature) |
| 11 and earlier | ❌ **Not Supported** | pgrx does not support these versions |

**Note:** While the extension supports PostgreSQL 13-17 via feature flags, only PostgreSQL 17 is actively tested in CI/CD. Other versions should work but haven't been validated.

### System Requirements

- **Rust**: 1.70+ (stable toolchain recommended)
- **pgrx**: 0.12.8
- **OS**: Linux (tested), macOS (should work), Windows (untested)
- **Disk Space**: ~100MB for build artifacts

### Build Dependencies

**Debian/Ubuntu:**

```bash
sudo apt-get install postgresql-server-dev-17 build-essential libclang-dev
```

**Arch Linux:**

```bash
sudo pacman -S postgresql-libs base-devel clang
```

**macOS:**

```bash
brew install postgresql@17 llvm
```

### Building for Different PostgreSQL Versions

```bash
# PostgreSQL 17 (default)
cargo pgrx install --release

# PostgreSQL 16
cargo pgrx install --release --pg-config /usr/lib/postgresql/16/bin/pg_config

# Or set in Cargo.toml:
# default = ["pg16"]
```

---

## 🤝 Contributing

This project is in alpha. Contributions, feedback, and bug reports are welcome!

- **Contributing Guide**: See [contributing.md](contributing.md) for development setup, code style, and PR guidelines
- **Bug Reports**: Open an issue at [GitHub Issues](https://github.com/fraiseql/jsonb_ivm/issues)
- **Questions**: Use [GitHub Discussions](https://github.com/fraiseql/jsonb_ivm/discussions)
- **Troubleshooting**: See [docs/troubleshooting.md](docs/troubleshooting.md)

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

### Next Steps

- Integration with pg_tview (replace manual refresh logic)
- Additional PostgreSQL version support (13-16)
- Further SIMD optimizations
- Production readiness hardening

---

Built with PostgreSQL ❤️ and Rust 🦀 | Alpha Quality | Performance Validated
