-- Deeply nested SQL - HAS ISSUES (missing closers)
-- Line comment trap: { [ ( } ] )
/* Block comment trap: { [ ( } ] ) */

-- String traps
SELECT '{ [ ( string trap } ] )' AS trap1;
SELECT $$ dollar trap: { [ ( } ] ) $$ AS trap2;

-- Structural issues
SELECT jsonb_build_object(
    'level1', jsonb_build_object(
        'level2', jsonb_build_object(
            'level3', jsonb_build_array(
                (SELECT id FROM nodes WHERE parent_id IS NULL),
                (SELECT id FROM nodes WHERE depth = 2
                -- missing ) for subquery
            )
        )
    )
);

WITH
    cte1 AS (
        SELECT id, data,
               (SELECT count(*) FROM children c WHERE c.parent_id = n.id) AS child_count
        FROM nodes n
    ),
    cte2 AS (
        SELECT c1.id,
               jsonb_build_object(
                   'count',    c1.child_count,
                   'children', (
                       SELECT jsonb_agg(jsonb_build_object('id', c.id, 'data', c.data)
                       FROM cte1 c
                       WHERE c.id = c1.id
                       -- missing ) for jsonb_agg and ) for subquery
                   )
               )
        FROM cte1 c1
    )
SELECT * FROM cte2;
