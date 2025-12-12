# jsonb_ivm

Surgical JSONB updates for PostgreSQL CQRS architectures

[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-13--18-blue.svg)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/License-PostgreSQL-blue.svg)](LICENSE)
[![Version](https://img.shields.io/badge/version-0.1.0-green)](changelog.md)

High-performance PostgreSQL extension for incremental JSONB view maintenance in CQRS/event sourcing systems. **2-7× faster** than native SQL re-aggregation.

---

## Why jsonb_ivm?

In CQRS architectures, you denormalize data into JSONB projection tables for read performance. When source data changes, you need to update these projections **surgically** without re-aggregating entire objects.

**The Problem**:

```sql
-- Re-aggregate entire object just to change one field 😢
UPDATE project_views
SET data = jsonb_build_object(
    'id', data->'id',
    'name', data->'name',
    'owner', jsonb_build_object(
        'id', data->'owner'->'id',
        'name', data->'owner'->'name',
        'email', 'alice@new.com'  -- only this changed!
    )
)
WHERE id = 1;
```

**The Solution**:

```sql
-- Surgical update with jsonb_ivm 🎯
UPDATE project_views
SET data = jsonb_smart_patch_nested(
    data,
    '{"email": "alice@new.com"}'::jsonb,
    ARRAY['owner']
)
WHERE id = 1;
```

**Result**: **2-3× faster**, cleaner code, less memory allocation.

---

## Features

- ✅ **Complete CRUD** for JSONB arrays (create, read, update, delete)
- ⚡ **2-7× faster** than native SQL re-aggregation
- 🎯 **Surgical updates** - modify only what changed
- 🛡️ **Null-safe** - graceful handling of missing paths/keys
- 🔧 **Production-ready** - extensively tested on PostgreSQL 13-18
- 📦 **Zero dependencies** - pure Rust with pgrx

---

## Quick Start

### Installation

```bash
# Build from source
git clone https://github.com/fraiseql/jsonb_ivm.git
cd jsonb_ivm
cargo install --locked cargo-pgrx
cargo pgrx init
cargo pgrx install --release
```

### Enable Extension

```sql
CREATE EXTENSION jsonb_ivm;
```

### Your First Query

```sql
-- Update array element by ID
UPDATE project_views
SET data = jsonb_smart_patch_array(
    data,
    '{"status": "completed"}'::jsonb,
    'tasks',    -- array path
    'id',       -- match key
    '5'::jsonb  -- match value
)
WHERE id = 1;
```

---

## Performance

| Operation | Native SQL | jsonb_ivm | Speedup |
|-----------|-----------|-----------|---------|
| Array element update | 3.2 ms | 1.1 ms | **2.9×** |
| Array DELETE | 4.1 ms | 0.6 ms | **6.8×** |
| Batch update (10 items) | 32 ms | 6 ms | **5.2×** |
| Multi-row (100 rows) | 450 ms | 110 ms | **4.1×** |

**See**: [Performance Benchmarks](docs/PERFORMANCE.md) for detailed analysis and methodology

---

## API Overview

### Core Functions

- `jsonb_merge_shallow(target, source)` - Fast top-level merge
- `jsonb_merge_at_path(target, source, path)` - Merge at nested path
- `jsonb_deep_merge(target, source)` - Recursive deep merge

### Array Updates

- `jsonb_array_update_where(target, array_path, match_key, match_value, updates)` - Update single element
- `jsonb_array_update_where_batch(target, array_path, match_key, updates_array)` - Batch updates
- `jsonb_array_update_multi_row(targets[], array_path, match_key, match_value, updates)` - Multi-row updates

### Array CRUD

- `jsonb_array_insert_where(target, array_path, element, sort_key, order)` - Sorted insertion
- `jsonb_array_delete_where(target, array_path, match_key, match_value)` - Delete element
- `jsonb_array_contains_id(data, array_path, key, value)` - Check existence

### Smart Patch Functions

- `jsonb_smart_patch_scalar(target, source)` - Intelligent shallow merge
- `jsonb_smart_patch_nested(target, source, path)` - Merge at nested path
- `jsonb_smart_patch_array(target, source, array_path, match_key, match_value)` - Update array element

**See**: [API Reference](docs/API.md) for complete function documentation with examples

---

## Documentation

- **[API Reference](docs/API.md)** - Complete function reference with examples
- **[Performance Benchmarks](docs/PERFORMANCE.md)** - Detailed benchmarks and methodology
- **[Architecture](docs/ARCHITECTURE.md)** - Design decisions and technical details
- **[Rust API Docs](https://docs.rs/jsonb_ivm)** - Generated from source code (coming soon)

---

## Use Cases

### CQRS Projection Maintenance

Update denormalized views when source entities change without re-aggregating.

### Event Sourcing

Incrementally update materialized views from event streams.

### pg_tview Integration

Optimize materialized view maintenance with surgical JSONB updates.

**See**: [Integration Guide](docs/integration-guide.md) for detailed examples

---

## Requirements

- PostgreSQL 13-18
- Rust 1.70+ (for building from source)
- pgrx 0.16.1

---

## Development

```bash
# Install task runner
cargo install just

# Run tests
just test

# Run benchmarks
just benchmark

# Format code
cargo fmt

# Lint
cargo clippy --all-targets --all-features -- -D warnings
```

**See**: [development.md](development.md) for detailed development guide

---

## Contributing

Contributions welcome! Please see:

- **[Contributing Guide](contributing.md)** - Development workflow, code standards
- **[Testing Guide](TESTING.md)** - How to run tests (why `cargo test` doesn't work)

---

## License

This project is licensed under the **PostgreSQL License** - see [LICENSE](LICENSE) for details.

---

## Changelog

See [CHANGELOG.md](changelog.md) for version history.

**Latest**: v0.1.0 - Initial release with complete JSONB CRUD operations

---

## Author

**Lionel Hamayon** - [fraiseql](https://github.com/fraiseql)

---

Built with PostgreSQL ❤️ and Rust 🦀
