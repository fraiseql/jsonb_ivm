-- Upgrade from jsonb_ivm v0.2.0 to v0.3.0
-- Adds pg_tview integration helpers: smart patch, array CRUD, deep merge, and helper functions

-- complain if script is sourced in psql, rather than via ALTER EXTENSION
\echo Use "ALTER EXTENSION jsonb_ivm UPDATE TO '0.3.0'" to load this file. \quit

-- Smart Patch Functions (Phase 1)

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

-- Array CRUD Operations (Phase 2)

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

-- Deep Merge & Helper Functions (Phase 3)

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
