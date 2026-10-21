WITH t0 AS (
  SELECT DISTINCT sm0.occurred_at AS occurred_at
  FROM stock_movements sm0
  WHERE sm0.kind = 'adjustment'
    AND sm0.occurred_at >= DATE '2025-01-01'
    AND sm0.occurred_at < DATE '2026-01-01'
),
t1 AS (
  SELECT sm1.occurred_at AS occurred_at,
         COUNT(*) AS event_count
  FROM stock_movements sm1
  WHERE sm1.kind = 'adjustment'
    AND sm1.occurred_at >= DATE '2025-01-01'
    AND sm1.occurred_at < DATE '2026-01-01'
  GROUP BY sm1.occurred_at
),
t2 AS (
  SELECT sm2.occurred_at AS occurred_at,
         COUNT(DISTINCT sl2.warehouse_id) AS distinct_warehouses
  FROM stock_movements sm2
  JOIN stock_locations sl2 ON sl2.id = sm2.stock_location_id
  WHERE sm2.kind = 'adjustment'
    AND sm2.occurred_at >= DATE '2025-01-01'
    AND sm2.occurred_at < DATE '2026-01-01'
  GROUP BY sm2.occurred_at
),
t3 AS (
  SELECT sm3.occurred_at AS occurred_at,
         COUNT(DISTINCT w3.tenant_id) AS distinct_tenants
  FROM stock_movements sm3
  JOIN stock_locations sl3 ON sl3.id = sm3.stock_location_id
  JOIN warehouses w3 ON w3.id = sl3.warehouse_id
  WHERE sm3.kind = 'adjustment'
    AND sm3.occurred_at >= DATE '2025-01-01'
    AND sm3.occurred_at < DATE '2026-01-01'
  GROUP BY sm3.occurred_at
),
t4 AS (
  SELECT sm4.occurred_at AS occurred_at,
         SUM(ABS(sm4.quantity)) AS total_abs_quantity
  FROM stock_movements sm4
  WHERE sm4.kind = 'adjustment'
    AND sm4.occurred_at >= DATE '2025-01-01'
    AND sm4.occurred_at < DATE '2026-01-01'
  GROUP BY sm4.occurred_at
)
SELECT
  t0.occurred_at AS occurred_at,
  COALESCE(t1.event_count, 0) AS event_count,
  COALESCE(t3.distinct_tenants, 0) AS distinct_tenants,
  COALESCE(t2.distinct_warehouses, 0) AS distinct_warehouses,
  COALESCE(t4.total_abs_quantity, 0) AS total_abs_quantity
FROM t0
LEFT JOIN t1 ON t1.occurred_at = t0.occurred_at
LEFT JOIN t2 ON t2.occurred_at = t0.occurred_at
LEFT JOIN t3 ON t3.occurred_at = t0.occurred_at
LEFT JOIN t4 ON t4.occurred_at = t0.occurred_at
ORDER BY event_count DESC NULLS FIRST, occurred_at ASC NULLS LAST
