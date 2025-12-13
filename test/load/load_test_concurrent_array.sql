-- Load test for concurrent array operations
-- This script tests the performance and correctness of array update operations
-- under concurrent load with 100 clients

\set id random(1, 1000)
\set random_val random(100, 999)

-- Update array element in test data
UPDATE test_jsonb
SET data = jsonb_ivm_array_update_where(
    data,
    'items',
    'id', :id::jsonb,
    'value', :random_val::jsonb
)
WHERE id = :id;
