# jsonb_ivm - Incremental JSONB View Maintenance

[![PostgreSQL](https://img.shields.io/badge/PostgreSQL-13%2B-blue.svg)](https://www.postgresql.org/)
[![License](https://img.shields.io/badge/License-PostgreSQL-blue.svg)](LICENSE)
[![Test](https://github.com/fraiseql/jsonb_ivm/actions/workflows/test.yml/badge.svg)](https://github.com/fraiseql/jsonb_ivm/actions/workflows/test.yml)
[![Lint](https://github.com/fraiseql/jsonb_ivm/actions/workflows/lint.yml/badge.svg)](https://github.com/fraiseql/jsonb_ivm/actions/workflows/lint.yml)

**High-performance PostgreSQL extension for intelligent partial updates of JSONB materialized views in CQRS architectures.**

> ⚠️ **Alpha Release**: This is v0.1.0-alpha1. API may change. Not recommended for production use yet.

---

## 🚀 Quick Start

### Installation

**From source (requires Rust):**

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

# Load extension in your database
psql -d your_database -c "CREATE EXTENSION jsonb_ivm;"
```

**From binary release (PostgreSQL 13-17):**

```bash
# Download release for your PostgreSQL version
wget https://github.com/fraiseql/jsonb_ivm/releases/download/v0.1.0-alpha1/jsonb_ivm-v0.1.0-alpha1-pg17.tar.gz

# Extract to PostgreSQL extension directory
sudo tar xzf jsonb_ivm-v0.1.0-alpha1-pg17.tar.gz -C $(pg_config --sharedir)/extension

# Load extension
psql -d your_database -c "CREATE EXTENSION jsonb_ivm;"
```

### Usage

```sql
-- Merge JSONB objects (shallow)
SELECT jsonb_merge_shallow(
    '{"a": 1, "b": 2}'::jsonb,
    '{"b": 99, "c": 3}'::jsonb
);
-- → {"a": 1, "b": 99, "c": 3}

-- Use in triggers for incremental view updates
CREATE TRIGGER sync_customer_updates
AFTER UPDATE ON tb_customers
FOR EACH ROW
EXECUTE FUNCTION update_tv_orders();

CREATE FUNCTION update_tv_orders()
RETURNS TRIGGER AS $$
BEGIN
    UPDATE tv_orders
    SET data = jsonb_merge_shallow(
        data,
        jsonb_build_object(
            'customer_name', NEW.name,
            'customer_email', NEW.email
        )
    )
    WHERE customer_id = NEW.id;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
```

---

## 📦 Features

### v0.1.0-alpha1
- ✅ `jsonb_merge_shallow(target, source)` - Shallow JSONB merge
- ✅ PostgreSQL 13-17 compatible
- ✅ IMMUTABLE, PARALLEL SAFE for query optimization
- ✅ Comprehensive test suite (12 tests)
- ✅ CI/CD on multiple PostgreSQL versions
- ✅ Zero compiler warnings enforcement
- ✅ Automated code quality checks

### Coming in Future Alphas
- ⏳ Nested path merge (`jsonb_merge_at_path`)
- ⏳ Change detection (`jsonb_detect_changes`)
- ⏳ Scope building system
- ⏳ Complete CQRS integration examples

---

## 🧪 Testing

```bash
# Run test suite
make installcheck

# All tests passing: 12/12 ✅
```

Test suite covers:
- Basic merge operations
- NULL handling
- Empty objects
- Overlapping keys
- Nested objects (shallow replacement)
- Different value types (strings, numbers, booleans, arrays, objects)
- Large objects (150 keys)
- Unicode support (emoji, international characters)
- Type validation errors

---

## 🛠️ Requirements

- **PostgreSQL**: 13 or later (tested on 13-17)
- **OS**: Linux, macOS
- **Compiler**: GCC 4.9+ or Clang 3.4+
- **Build Tools**: make, PostgreSQL dev headers

### Installing Build Dependencies

**Debian/Ubuntu:**
```bash
sudo apt-get install postgresql-server-dev-17 build-essential
```

**RHEL/CentOS:**
```bash
sudo yum install postgresql17-devel gcc make
```

**Arch Linux:**
```bash
sudo pacman -S postgresql-libs base-devel
```

---

## 📖 API Reference

### `jsonb_merge_shallow(target, source)`

Merges top-level keys from `source` JSONB into `target` JSONB.

**Parameters:**
- `target` (jsonb) - Base JSONB object to merge into
- `source` (jsonb) - JSONB object whose keys will be merged

**Returns:**
- `jsonb` - New JSONB object with merged keys

**Behavior:**
- Source keys **overwrite** target keys on conflicts
- Returns `NULL` if either argument is `NULL`
- Raises error if either argument is not a JSONB object (arrays/scalars rejected)
- **Shallow merge**: Nested objects are replaced entirely, not recursively merged

**Examples:**

```sql
-- Basic merge
SELECT jsonb_merge_shallow('{"a": 1}'::jsonb, '{"b": 2}'::jsonb);
-- → {"a": 1, "b": 2}

-- Overwrite on conflict
SELECT jsonb_merge_shallow('{"a": 1, "b": 2}'::jsonb, '{"b": 99}'::jsonb);
-- → {"a": 1, "b": 99}

-- Shallow merge (nested object replaced)
SELECT jsonb_merge_shallow(
    '{"user": {"name": "John", "age": 30}}'::jsonb,
    '{"user": {"email": "john@example.com"}}'::jsonb
);
-- → {"user": {"email": "john@example.com"}}  (age lost!)
```

**Performance:**
- Delegates to PostgreSQL's internal `jsonb_concat` operator
- O(n + m) where n = target keys, m = source keys
- Minimal memory overhead

---

## 🤝 Contributing

This project is in alpha. We welcome **feedback and bug reports** but are not yet accepting code contributions.

**Found a bug?** Open an issue: https://github.com/fraiseql/jsonb_ivm/issues

**Want a feature?** Open a discussion: https://github.com/fraiseql/jsonb_ivm/discussions

---

## 📋 Changelog

See [CHANGELOG.md](CHANGELOG.md) for version history.

---

## 📜 License

Licensed under the PostgreSQL License. See [LICENSE](LICENSE) for details.

---

## 👤 Author

**Lionel Hamayon** - [fraiseql](https://github.com/fraiseql)

---

## 🏗️ Development Philosophy

This project follows a **quality-first, CI/CD-driven** development methodology:

- ✅ **Tests before code** (TDD)
- ✅ **Zero compiler warnings** enforced
- ✅ **Automated quality gates** on every commit
- ✅ **Multi-version PostgreSQL testing** (13-17)
- ✅ **Incremental alpha releases** with clear scope
- ✅ **Documentation-driven development**

**Built with PostgreSQL ❤️  | Alpha Quality | Battle-tested with automated CI/CD**
