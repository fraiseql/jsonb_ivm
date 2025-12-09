-- jsonb_ivm extension v0.3.0
-- Incremental JSONB View Maintenance for CQRS Architectures
-- Copyright (c) 2025 Lionel Hamayon
-- Licensed under the PostgreSQL License

-- complain if script is sourced in psql, rather than via CREATE EXTENSION
\echo Use "CREATE EXTENSION jsonb_ivm" to load this file. \quit

-- Core Functions (v0.1.0)

-- jsonb_merge_shallow: Shallow merge of two JSONB objects
CREATE FUNCTION jsonb_merge_shallow(
	target jsonb,
	source jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_merge_shallow_wrapper';

COMMENT ON FUNCTION jsonb_merge_shallow(jsonb, jsonb) IS
'Shallow merge of two JSONB objects. Source keys overwrite target keys on conflict.';

-- jsonb_array_update_where: Update single element in JSONB array
CREATE FUNCTION jsonb_array_update_where(
	target jsonb,
	array_path TEXT,
	match_key TEXT,
	match_value jsonb,
	updates jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_array_update_where_wrapper';

COMMENT ON FUNCTION jsonb_array_update_where(jsonb, TEXT, TEXT, jsonb, jsonb) IS
'Update a single element in a JSONB array by matching a key-value predicate. 2-3× faster than native SQL re-aggregation.';

-- jsonb_merge_at_path: Merge JSONB at nested path
CREATE FUNCTION jsonb_merge_at_path(
	target jsonb,
	source jsonb,
	path TEXT[]
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_merge_at_path_wrapper';

COMMENT ON FUNCTION jsonb_merge_at_path(jsonb, jsonb, TEXT[]) IS
'Merge a JSONB object at a specific nested path within the target document.';

-- Performance Functions (v0.2.0)

-- jsonb_array_update_where_batch: Batch update multiple array elements
CREATE FUNCTION jsonb_array_update_where_batch(
	target jsonb,
	array_path TEXT,
	match_key TEXT,
	updates_array jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_array_update_where_batch_wrapper';

COMMENT ON FUNCTION jsonb_array_update_where_batch(jsonb, TEXT, TEXT, jsonb) IS
'Batch update multiple elements in a JSONB array. 3-5× faster than multiple separate calls.';

-- jsonb_array_update_multi_row: Multi-row array updates
CREATE FUNCTION jsonb_array_update_multi_row(
	targets jsonb[],
	array_path TEXT,
	match_key TEXT,
	match_value jsonb,
	updates jsonb
) RETURNS TABLE (result jsonb)
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_array_update_multi_row_wrapper';

COMMENT ON FUNCTION jsonb_array_update_multi_row(jsonb[], TEXT, TEXT, jsonb, jsonb) IS
'Update arrays across multiple JSONB documents in one call. ~4× faster for 100-row batches.';

-- pg_tview Integration Helpers (v0.3.0)

-- Smart Patch Functions

CREATE FUNCTION jsonb_smart_patch_scalar(
	target jsonb,
	source jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_smart_patch_scalar_wrapper';

COMMENT ON FUNCTION jsonb_smart_patch_scalar(jsonb, jsonb) IS
'Intelligent shallow merge for top-level object updates. Simplifies pg_tview refresh logic.';

CREATE FUNCTION jsonb_smart_patch_nested(
	target jsonb,
	source jsonb,
	path TEXT[]
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_smart_patch_nested_wrapper';

COMMENT ON FUNCTION jsonb_smart_patch_nested(jsonb, jsonb, TEXT[]) IS
'Merge JSONB at nested path within document. Replaces complex path manipulation logic.';

CREATE FUNCTION jsonb_smart_patch_array(
	target jsonb,
	source jsonb,
	array_path TEXT,
	match_key TEXT,
	match_value jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_smart_patch_array_wrapper';

COMMENT ON FUNCTION jsonb_smart_patch_array(jsonb, jsonb, TEXT, TEXT, jsonb) IS
'Update specific element within JSONB array. Optimized for pg_tview cascade patterns.';

-- Array CRUD Operations

CREATE FUNCTION jsonb_array_delete_where(
	target jsonb,
	array_path TEXT,
	match_key TEXT,
	match_value jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_array_delete_where_wrapper';

COMMENT ON FUNCTION jsonb_array_delete_where(jsonb, TEXT, TEXT, jsonb) IS
'Surgically delete an element from a JSONB array. 3-5× faster than re-aggregation.';

CREATE FUNCTION jsonb_array_insert_where(
	target jsonb,
	array_path TEXT,
	new_element jsonb,
	sort_key TEXT,
	sort_order TEXT
) RETURNS jsonb
IMMUTABLE PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_array_insert_where_wrapper';

COMMENT ON FUNCTION jsonb_array_insert_where(jsonb, TEXT, jsonb, TEXT, TEXT) IS
'Insert element into JSONB array with optional sorting. 3-5× faster than re-aggregation. sort_key and sort_order are optional (can be NULL).';

-- Deep Merge & Helper Functions

CREATE FUNCTION jsonb_deep_merge(
	target jsonb,
	source jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_deep_merge_wrapper';

COMMENT ON FUNCTION jsonb_deep_merge(jsonb, jsonb) IS
'Recursively merge nested JSONB objects, preserving fields not present in source. 2× faster than multiple jsonb_merge_at_path calls.';

CREATE FUNCTION jsonb_extract_id(
	data jsonb,
	key TEXT DEFAULT 'id'
) RETURNS TEXT
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_extract_id_wrapper';

COMMENT ON FUNCTION jsonb_extract_id(jsonb, TEXT) IS
'Safely extract an ID field from JSONB as text. Defaults to extracting the "id" key.';

CREATE FUNCTION jsonb_array_contains_id(
	data jsonb,
	array_path TEXT,
	id_key TEXT,
	id_value jsonb
) RETURNS bool
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_array_contains_id_wrapper';

COMMENT ON FUNCTION jsonb_array_contains_id(jsonb, TEXT, TEXT, jsonb) IS
'Fast check if a JSONB array contains an element with a specific ID. Uses loop unrolling optimization.';
