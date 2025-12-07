-- jsonb_ivm extension version 0.1.0

-- Merge top-level keys from source JSONB into target JSONB (shallow merge)
CREATE FUNCTION jsonb_merge_shallow(
    target jsonb,
    source jsonb
) RETURNS jsonb
IMMUTABLE STRICT PARALLEL SAFE
LANGUAGE c
AS 'MODULE_PATHNAME', 'jsonb_merge_shallow_wrapper';
