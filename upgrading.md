# Upgrading jsonb_ivm

Guide for upgrading between versions of jsonb_ivm.

---

## Table of Contents

- [Upgrading from v0.2.0 to v0.3.0](#upgrading-from-v020-to-v030)
- [Upgrade Procedure](#upgrade-procedure)
- [Breaking Changes](#breaking-changes)
- [New Features](#new-features)
- [Migration Examples](#migration-examples)
- [Rollback Procedure](#rollback-procedure)

---

## Upgrading from v0.2.0 to v0.3.0

### Overview

v0.3.0 is **100% backward compatible** with v0.2.0. All existing functions continue to work unchanged.

**What's New:**
- 8 new functions (13 total)
- Complete JSONB array CRUD support (INSERT and DELETE operations)
- Smart patch API for simplified dispatch logic
- Deep merge for recursive nested updates
- Helper functions for ID extraction and containment checks

**Performance Impact:**
- No performance regression on existing functions
- New functions provide 3-7× speedups for INSERT/DELETE operations

---

## Upgrade Procedure

### Method 1: Extension Update (Recommended)

```sql
-- Check current version
SELECT * FROM pg_extension WHERE extname = 'jsonb_ivm';
-- Should show version 0.2.0

-- Upgrade to v0.3.0
ALTER EXTENSION jsonb_ivm UPDATE TO '0.3.0';

-- Verify upgrade
SELECT * FROM pg_extension WHERE extname = 'jsonb_ivm';
-- Should show version 0.3.0

-- Test a new function
SELECT jsonb_smart_patch_scalar('{"a": 1}'::jsonb, '{"b": 2}'::jsonb);
-- Result: {"a": 1, "b": 2}
```

**Requirements:**
- Extension must be installed from v0.2.0
- Upgrade path file `jsonb_ivm--0.2.0--0.3.0.sql` must be present

---

### Method 2: Clean Install (If upgrade fails)

```sql
-- 1. Drop existing extension (WARNING: removes all functions)
DROP EXTENSION jsonb_ivm CASCADE;

-- 2. Reinstall from source
-- (On server: cargo pgrx install --release)

-- 3. Create extension (new version)
CREATE EXTENSION jsonb_ivm;

-- 4. Verify installation
SELECT * FROM pg_extension WHERE extname = 'jsonb_ivm';
-- Should show version 0.3.0
```

**IMPORTANT:** This will drop all existing functions. If you have views or functions that depend on jsonb_ivm, they will be dropped with CASCADE.

---

### Method 3: Side-by-Side Install (Testing)

```sql
-- Install in a test database
CREATE DATABASE jsonb_ivm_test;
\c jsonb_ivm_test

CREATE EXTENSION jsonb_ivm;

-- Test new functions
SELECT jsonb_array_insert_where(
    '{"posts": []}'::jsonb,
    'posts',
    '{"id": 1, "title": "Test"}'::jsonb,
    NULL,
    NULL
);

-- If satisfied, upgrade production database
```

---

## Breaking Changes

### None

v0.3.0 has **zero breaking changes**. All v0.2.0 functions work identically.

---

## New Features

### 1. Smart Patch Functions (Simplified Dispatch)

**Before (v0.2.0):**
```sql
-- Had to manually choose between jsonb_merge_shallow and jsonb_merge_at_path
UPDATE tv_user
SET data = CASE
    WHEN updates_type = 'scalar' THEN jsonb_merge_shallow(data, updates)
    WHEN updates_type = 'nested' THEN jsonb_merge_at_path(data, updates, path)
    ELSE data
END;
```

**After (v0.3.0):**
```sql
-- Unified smart patch API
UPDATE tv_user
SET data = jsonb_smart_patch_scalar(data, updates);  -- For top-level updates

UPDATE tv_user
SET data = jsonb_smart_patch_nested(data, updates, ARRAY['company']);  -- For nested

UPDATE tv_feed
SET data = jsonb_smart_patch_array(data, updates, 'posts', 'id', post_id);  -- For arrays
```

**Benefit:** Clearer intent, 40-60% code reduction in refresh logic

---

### 2. Array DELETE Operations (Was Missing)

**Before (v0.2.0 - Only option was re-aggregation):**
```sql
-- Delete post from feed → re-aggregate entire array
UPDATE tv_feed
SET data = jsonb_build_object(
    'posts',
    (
        SELECT jsonb_agg(data ORDER BY created_at DESC)
        FROM tv_post
        WHERE pk != deleted_post_pk
        LIMIT 100
    )
);
-- Time: ~20-30ms for 100 posts
```

**After (v0.3.0 - Surgical deletion):**
```sql
-- Surgically remove post from array
UPDATE tv_feed
SET data = jsonb_array_delete_where(
    data,
    'posts',
    'id',
    deleted_post_id
)
WHERE jsonb_array_contains_id(data, 'posts', 'id', deleted_post_id);
-- Time: ~4-6ms (5× faster)
```

---

### 3. Array INSERT Operations (Was Missing)

**Before (v0.2.0 - Only option was re-aggregation):**
```sql
-- Add new post → re-aggregate entire array with sorting
UPDATE tv_feed
SET data = jsonb_build_object(
    'posts',
    (
        SELECT jsonb_agg(data ORDER BY created_at DESC)
        FROM tv_post
        LIMIT 100
    )
);
-- Time: ~22-35ms for 100 posts
```

**After (v0.3.0 - Sorted insertion):**
```sql
-- Insert post in correct sorted position
UPDATE tv_feed
SET data = jsonb_array_insert_where(
    data,
    'posts',
    new_post_data,
    'created_at',  -- sort by this field
    'DESC'         -- descending order
);
-- Time: ~5-8ms (4-6× faster)
```

---

### 4. Deep Merge (Recursive nested updates)

**Before (v0.2.0 - Multiple jsonb_merge_at_path calls):**
```sql
-- Update nested company data in posts (3 levels deep)
UPDATE tv_post
SET data = jsonb_merge_at_path(
    jsonb_merge_at_path(
        data,
        new_company_name,
        ARRAY['author', 'company', 'name']
    ),
    new_company_industry,
    ARRAY['author', 'company', 'industry']
);
-- Complex, error-prone, multiple function calls
```

**After (v0.3.0 - Single deep merge):**
```sql
-- Recursively merge at any depth
UPDATE tv_post
SET data = jsonb_deep_merge(
    data,
    jsonb_build_object(
        'author',
        jsonb_build_object(
            'company',
            new_company_data  -- All company fields at once
        )
    )
);
-- Simple, preserves unchanged nested fields, 2× faster
```

---

### 5. Helper Functions

**New in v0.3.0:**

```sql
-- Extract ID from JSONB (default key: 'id')
SELECT jsonb_extract_id('{"id": "abc-123", "name": "Test"}'::jsonb);
-- Result: "abc-123"

-- Extract custom key
SELECT jsonb_extract_id(data, 'user_id') FROM tv_post;

-- Fast containment check
SELECT pk FROM tv_feed
WHERE jsonb_array_contains_id(data, 'posts', 'id', target_post_id);
-- Much faster than jsonb_array_elements + WHERE clause
```

---

## Migration Examples

### Migrating DELETE Operations

**Old approach (v0.2.0):**
```sql
-- Trigger on post deletion
CREATE OR REPLACE FUNCTION propagate_post_deletion_v02()
RETURNS TRIGGER AS $$
BEGIN
    -- Re-aggregate entire feed array
    UPDATE tv_feed
    SET data = jsonb_build_object(
        'posts',
        (
            SELECT jsonb_agg(data ORDER BY (data->>'created_at')::timestamptz DESC)
            FROM tv_post
            WHERE (data->>'id')::uuid != (OLD.data->>'id')::uuid
            LIMIT 100
        )
    )
    WHERE pk = 1;  -- Simplification: assuming single feed

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;
```

**New approach (v0.3.0):**
```sql
-- Trigger on post deletion (surgical)
CREATE OR REPLACE FUNCTION propagate_post_deletion_v03()
RETURNS TRIGGER AS $$
BEGIN
    -- Surgically remove from affected feeds
    UPDATE tv_feed
    SET data = jsonb_array_delete_where(
        data,
        'posts',
        'id',
        OLD.data->'id'
    )
    WHERE jsonb_array_contains_id(data, 'posts', 'id', OLD.data->'id');

    RETURN OLD;
END;
$$ LANGUAGE plpgsql;
```

**Benefits:**
- 5× faster execution
- Works with multiple feeds automatically (no hardcoded pk = 1)
- Cleaner code

---

### Migrating INSERT Operations

**Old approach (v0.2.0):**
```sql
-- After inserting post, update feed
INSERT INTO tv_post VALUES (...);

UPDATE tv_feed
SET data = jsonb_build_object(
    'posts',
    (
        SELECT jsonb_agg(data ORDER BY (data->>'created_at')::timestamptz DESC)
        FROM tv_post
        LIMIT 100
    )
);
```

**New approach (v0.3.0):**
```sql
-- After inserting post, surgically add to feed
INSERT INTO tv_post VALUES (...);

UPDATE tv_feed
SET data = jsonb_array_insert_where(
    data,
    'posts',
    (SELECT data FROM tv_post WHERE pk = NEW.pk),
    'created_at',
    'DESC'
);
```

**Benefits:**
- 4-6× faster
- Maintains sort order automatically
- No risk of re-sorting bugs

---

### Migrating Nested Updates

**Old approach (v0.2.0):**
```sql
-- Update company in deeply nested structure
UPDATE tv_post
SET data = jsonb_set(
    jsonb_set(
        data,
        '{author,company,name}',
        new_name
    ),
    '{author,company,industry}',
    new_industry
)
WHERE fk_user IN (SELECT pk FROM tv_user WHERE fk_company = 1);
```

**New approach (v0.3.0):**
```sql
-- Deep merge entire company object
UPDATE tv_post
SET data = jsonb_deep_merge(
    data,
    jsonb_build_object(
        'author',
        jsonb_build_object(
            'company',
            (SELECT data FROM v_company WHERE pk = 1)
        )
    )
)
WHERE fk_user IN (SELECT pk FROM tv_user WHERE fk_company = 1);
```

**Benefits:**
- Updates all company fields in one call
- Preserves unchanged nested fields
- 2× faster than multiple jsonb_set() calls

---

## Rollback Procedure

If you need to rollback to v0.2.0:

### Option 1: Downgrade Extension (Not Supported)

There is no automatic downgrade path from v0.3.0 to v0.2.0.

### Option 2: Reinstall v0.2.0

```bash
# On server:
# 1. Checkout v0.2.0 tag
git checkout v0.2.0

# 2. Rebuild and install
cargo pgrx install --release

# 3. In PostgreSQL:
DROP EXTENSION jsonb_ivm CASCADE;
CREATE EXTENSION jsonb_ivm;  -- Will create v0.2.0

# 4. Verify version
SELECT * FROM pg_extension WHERE extname = 'jsonb_ivm';
-- Should show version 0.2.0
```

**WARNING:** This will remove v0.3.0 functions. If your code uses them, it will break.

---

## Post-Upgrade Checklist

After upgrading to v0.3.0:

- [ ] Verify extension version: `SELECT * FROM pg_extension WHERE extname = 'jsonb_ivm'`
- [ ] Test a new function: `SELECT jsonb_smart_patch_scalar('{"a":1}'::jsonb, '{"b":2}'::jsonb)`
- [ ] Run existing queries (should work unchanged)
- [ ] Review code for optimization opportunities (see [Migration Examples](#migration-examples))
- [ ] Update any manual re-aggregation queries to use new INSERT/DELETE functions
- [ ] Consider replacing nested `jsonb_set()` calls with `jsonb_deep_merge()`
- [ ] Update documentation/comments referencing v0.2.0

---

## Getting Help

- **Troubleshooting**: See [docs/troubleshooting.md](docs/troubleshooting.md)
- **Integration Examples**: See [docs/pg-tview-integration-examples.md](docs/pg-tview-integration-examples.md)
- **Issues**: [GitHub Issues](https://github.com/fraiseql/jsonb_ivm/issues)

---

## Version History

| Version | Release Date | Key Changes |
|---------|--------------|-------------|
| **v0.3.0** | 2025-12-09 | pg_tview integration helpers, complete CRUD, deep merge |
| **v0.2.0** | 2025-12-08 | Performance optimizations, SIMD, batch functions |
| **v0.1.0** | 2025-12-07 | Initial release, 3 core functions |

---

**Upgrade complete! 🚀**

See [changelog.md](changelog.md) for full release notes.
