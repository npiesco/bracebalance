-- Deeply nested SQL with PL/pgSQL dollar-quoting - BALANCED
-- Line comment trap: { [ ( } ] )
/* Block comment trap: { [ ( } ] ) */

-- String traps
SELECT '{ [ ( string trap } ] )' AS trap1;
SELECT $$ dollar trap: { [ ( } ] ) $$ AS trap2;
SELECT $body$ body-tag dollar trap: { [ ( } ] ) $body$ AS trap3;

-- PL/pgSQL function with dollar-quoting (all braces inside are traps)
CREATE OR REPLACE FUNCTION deep_process(p_config jsonb)
RETURNS jsonb
LANGUAGE plpgsql
AS $$
DECLARE
    v_result jsonb;
    v_items  jsonb;
BEGIN
    -- Internal comment: { [ ( trap } ] )
    SELECT jsonb_build_object(
        'connections', (
            SELECT jsonb_agg(
                jsonb_build_object(
                    'host', c->>'host',
                    'options', jsonb_build_object(
                        'pool', jsonb_build_object(
                            'min', (c->'pool'->>'min')::int,
                            'max', (c->'pool'->>'max')::int
                        )
                    )
                )
            )
            FROM jsonb_array_elements(p_config->'connections') c
        )
    ) INTO v_result;
    RETURN v_result;
END;
$$;

-- Nested CTEs (structural braces must count correctly)
WITH RECURSIVE
    level1 AS (
        SELECT id, parent_id, data
        FROM   nodes
        WHERE  parent_id IS NULL
    ),
    level2 AS (
        SELECT n.id, n.parent_id, n.data,
               jsonb_build_object(
                   'parent', l.data,
                   'self',   n.data
               ) AS combined
        FROM   nodes n
        JOIN   level1 l ON l.id = n.parent_id
    ),
    level3 AS (
        SELECT l2.id,
               jsonb_build_object(
                   'level', 3,
                   'ancestry', jsonb_build_array(
                       l2.combined->'parent',
                       l2.combined->'self'
                   )
               ) AS full_path
        FROM   level2 l2
    )
SELECT * FROM level3;
