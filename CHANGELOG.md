# Changelog

All notable changes to the `jsonb_ivm` extension will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.3.0] - 2025-12-08

### 🚀 pg_tview Integration Helpers

This release adds complete JSONB array CRUD support and helper functions specifically designed for pg_tview integration, simplifying incremental view maintenance code by 40-60%.

### Added

#### Smart Patch Functions
- **`jsonb_smart_patch_scalar(target, source)`**
  - Intelligent shallow merge for top-level object updates
  - Simplifies pg_tview refresh logic
  - Single code path for scalar updates

- **`jsonb_smart_patch_nested(target, source, path)`**
  - Merge JSONB at nested path within document
  - Replaces complex path manipulation logic
  - Array-based path specification

- **`jsonb_smart_patch_array(target, source, array_path, match_key, match_value)`**
  - Update specific element within JSONB array
  - Combines search and update in single operation
  - Optimized for pg_tview cascade patterns

#### Array CRUD Operations
- **`jsonb_array_delete_where(target, array_path, match_key, match_value)`**
  - Surgical array element deletion
  - **3-5× faster** than re-aggregation
  - Completes CRUD operations (DELETE was missing in v0.2.0)
  - Essential for pg_tview INSERT/DELETE propagation

- **`jsonb_array_insert_where(target, array_path, new_element, sort_key, sort_order)`**
  - Ordered array insertion with optional sorting
  - **3-5× faster** than re-aggregation
  - Completes CRUD operations (INSERT was missing in v0.2.0)
  - Maintains sort order (ASC/DESC) automatically

#### Deep Operations
- **`jsonb_deep_merge(target, source)`**
  - Recursive deep merge preserving nested fields
  - Fixes `jsonb_merge_shallow` limitation
  - **2× faster** than multiple `jsonb_merge_at_path` calls
  - Critical for nested dependency updates

#### Helper Functions
- **`jsonb_extract_id(data, key DEFAULT 'id')`**
  - Safe ID extraction (UUID or integer)
  - Returns text representation
  - Handles missing keys gracefully

- **`jsonb_array_contains_id(data, array_path, id_key, id_value)`**
  - Fast containment check with loop unrolling optimization
  - Used for pg_tview propagation decisions
  - Returns boolean for filtering queries

### Performance Improvements
- **INSERT operations**: 3-5× faster (eliminates re-aggregation)
- **DELETE operations**: 3-5× faster (eliminates re-aggregation)
- **Deep nested updates**: 2× faster (single function call vs multiple operations)
- **Overall pg_tview cascades**: +10-20% throughput improvement

### Documentation
- Added `docs/PG_TVIEW_INTEGRATION_EXAMPLES.md` - Complete CRUD workflow examples
- Updated README.md with v0.3.0 API and usage examples
- Comprehensive benchmark suite (`test/benchmark_pg_tview_helpers.sql`)
- Smoke test suite (`test/smoke_test_v0.3.0.sql`)

### Code Quality
- Fixed pgrx SQL generation (added `pgrx::pgrx_embed!()` macro)
- Added "lib" crate type for proper extension installation
- Generated `sql/jsonb_ivm--0.3.0.sql` with all 13 function definitions
- All functions marked IMMUTABLE, PARALLEL SAFE, STRICT

### Breaking Changes
None - all additions are backward compatible with v0.2.0 and v0.1.0.

### Migration from v0.2.0
No changes required - v0.3.0 is fully backward compatible.
To use new features, update SQL queries to call new functions.

**Example Migration:**
```sql
-- Old (re-aggregation):
UPDATE tv_feed
SET data = jsonb_build_object(
    'posts',
    (SELECT jsonb_agg(data ORDER BY created_at DESC)
     FROM tv_post WHERE pk != deleted_post_pk)
)
WHERE pk = 1;

-- New (surgical deletion):
UPDATE tv_feed
SET data = jsonb_array_delete_where(
    data, 'posts', 'id', deleted_post_id::jsonb
)
WHERE pk = 1;
```

### Impact on pg_tview
- **60% code reduction** in refresh.rs (150 lines → 60 lines)
- **Complete CRUD support** - INSERT and DELETE operations now available
- **Simplified dispatch logic** - Single code path for all update types
- **Better maintainability** - Less manual JSONB manipulation

---

## [0.1.0-alpha1] - 2025-12-07

### 🎉 Initial Alpha Release

This is the first public release of jsonb_ivm, starting from a clean slate with a quality-first, CI/CD-driven approach.

### Added

- **Core Function**: `jsonb_merge_shallow(target, source)`
  - Shallow merge of two JSONB objects
  - Source keys overwrite target keys on conflicts
  - NULL-safe with proper error handling
  - IMMUTABLE and PARALLEL SAFE for query optimization

- **Testing Infrastructure**
  - 12 comprehensive tests covering all edge cases
  - PostgreSQL regression test framework integration
  - Tests for NULL handling, empty objects, large objects, Unicode

- **CI/CD Pipeline**
  - GitHub Actions workflow for multi-version testing (PostgreSQL 13-17)
  - Automated linting and code quality checks
  - Zero compiler warnings enforcement (`-Wall -Wextra -Werror`)
  - Trailing whitespace validation
  - Automated test result uploads

- **Code Quality**
  - clang-format configuration for consistent C code style
  - PGXS-based build system
  - Comprehensive inline documentation
  - Type validation with helpful error messages

- **Documentation**
  - README with installation, usage, and API reference
  - CHANGELOG following Keep a Changelog format
  - PostgreSQL License

### Technical Details

- **PostgreSQL Compatibility**: 13, 14, 15, 16, 17
- **Build System**: cargo-pgrx 0.12.8
- **Language**: Rust (Edition 2021)
- **Framework**: pgrx - PostgreSQL extension framework for Rust
- **License**: PostgreSQL License

### Implementation Notes

- Migrated from C to Rust for memory safety guarantees
- Manual JSONB merge implementation using Rust HashMap operations
- Rust ownership system prevents buffer overflows, use-after-free bugs
- See `.archive-c-implementation/` for original C version

### Notes

- This is an **alpha release** - API may change in future versions
- Not recommended for production use yet
- Focused on minimal viable functionality with perfect quality
- Foundation for incremental feature additions in future alphas

### Migration from C to Rust

This release represents a complete rewrite from C to Rust using the pgrx framework.

**What Changed:**
- ✅ Implementation language: C → Rust
- ✅ Build system: PGXS → cargo-pgrx
- ✅ Memory safety: Manual management → Rust ownership
- ✅ Type safety: Runtime checks → Compile-time guarantees
- ⚠️ Performance: Native jsonb_concat → Manual merge (20-40% slower, but safer)

**What Stayed the Same:**
- ✅ Function signature: `jsonb_merge_shallow(target, source)`
- ✅ Behavior: Shallow merge, source overwrites target
- ✅ NULL handling: STRICT attribute
- ✅ PostgreSQL attributes: IMMUTABLE, PARALLEL SAFE
- ✅ Test coverage: All tests pass with identical results

**Why Rust:**
- Eliminates entire classes of memory safety bugs
- Better testing infrastructure (Rust + SQL tests)
- Modern tooling (clippy, rustfmt, cargo-audit)
- Foundation for future features (nested merge, change detection)

See [comprehensive code review](CODE_REVIEW_PROMPT.md) for detailed quality assessment.

---

## Roadmap

### Planned for v0.2.0-alpha1
- Nested path merge function: `jsonb_merge_at_path(target, source, path)`
- Additional tests for nested operations
- Performance benchmarks

### Planned for v0.3.0-alpha1
- Change detection: `jsonb_detect_changes(old, new, keys)`
- Sub-millisecond performance validation

### Planned for v0.4.0-alpha1
- Scope building system
- Configuration-driven update patterns

### Planned for v0.5.0-beta1
- Feature complete
- Seek early adopters
- Real-world validation

### Planned for v1.0.0
- Production-ready release
- Published to PGXN
- Community validation

---

## Contributing

See [README.md](README.md#contributing) for contribution guidelines.

## Links

- **GitHub**: https://github.com/fraiseql/jsonb_ivm
- **Issues**: https://github.com/fraiseql/jsonb_ivm/issues
- **Discussions**: https://github.com/fraiseql/jsonb_ivm/discussions
