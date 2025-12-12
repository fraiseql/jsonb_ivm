# jsonb_ivm

**Surgical JSONB updates for PostgreSQL CQRS architectures**

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

**See**: [Quick Start Guide](docs/quick-start.md) for full walkthrough

---

## Performance

| Operation | Native SQL | jsonb_ivm | Speedup |
|-----------|-----------|-----------|---------|
| Array element update | 1.91 ms | 0.72 ms | **2.66×** |
| Array DELETE | 20-30 ms | 4-6 ms | **5-7×** |
| Array INSERT (sorted) | 22-35 ms | 5-8 ms | **4-6×** |
| Deep merge | 8-12 ms | 4-6 ms | **2×** |

**See**: [Benchmark Results](docs/implementation/benchmark-results.md) for detailed analysis

---

## API Overview

### Smart Patch Functions

- `jsonb_smart_patch_scalar(target, source)` - Intelligent shallow merge
- `jsonb_smart_patch_nested(target, source, path)` - Merge at nested path
- `jsonb_smart_patch_array(target, source, array_path, match_key, match_value)` - Update array element

### Array CRUD

- `jsonb_array_insert_where(target, array_path, element, sort_key, order)` - Sorted insertion
- `jsonb_array_delete_where(target, array_path, match_key, match_value)` - Delete element
- `jsonb_array_contains_id(data, array_path, key, value)` - Check existence

### Deep Merge

- `jsonb_deep_merge(target, source)` - Recursive deep merge

**See**: [API Reference](docs/api-reference.md) for complete function documentation

---

## Documentation

- **[Quick Start Guide](docs/quick-start.md)** - Get up and running in 5 minutes
- **[API Reference](docs/api-reference.md)** - Complete function reference
- **[Integration Guide](docs/integration-guide.md)** - Real-world CRUD workflows
- **[Architecture](docs/architecture.md)** - Technical design and implementation
- **[Troubleshooting](docs/troubleshooting.md)** - Common issues and solutions

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
