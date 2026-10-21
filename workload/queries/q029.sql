WITH order_fulfillment_counts AS (
  SELECT order_id, COUNT(*) AS fulfillment_count
  FROM fulfillments
  GROUP BY order_id
),
qualifying_fulfillments AS (
  SELECT
    o.id AS order_id,
    o.placed_at AS placed_at,
    f.warehouse_id AS warehouse_id,
    f.shipped_at AS shipped_at,
    f.delivered_at AS delivered_at
  FROM orders o
  JOIN order_fulfillment_counts ofc ON ofc.order_id = o.id AND ofc.fulfillment_count = 1
  JOIN fulfillments f ON f.order_id = o.id
  WHERE o.status = 'fulfilled'
    AND o.placed_at >= DATE '2025-01-01'
    AND o.placed_at < DATE '2026-01-01'
    AND f.shipped_at IS NOT NULL
    AND f.delivered_at IS NOT NULL
),
order_metrics AS (
  SELECT
    warehouse_id,
    {{date_diff_days(placed_at, shipped_at)}} AS processing_days,
    {{date_diff_days(placed_at, delivered_at)}} AS delivery_days
  FROM qualifying_fulfillments
),
warehouse_agg AS (
  SELECT
    warehouse_id,
    COUNT(*) AS orders_shipped,
    AVG(CAST(processing_days AS NUMERIC(18,6))) AS avg_processing_days,
    AVG(CAST(delivery_days AS NUMERIC(18,6))) AS avg_delivery_days,
    AVG(CASE WHEN delivery_days <= 5 THEN 1.0 ELSE 0.0 END) * 100 AS pct_within_5_days
  FROM order_metrics
  GROUP BY warehouse_id
  HAVING COUNT(*) >= 30
)
SELECT
  w.name AS warehouse_name,
  t.name AS tenant_name,
  wa.orders_shipped AS orders_shipped,
  wa.avg_processing_days AS avg_processing_days,
  wa.avg_delivery_days AS avg_delivery_days,
  wa.pct_within_5_days AS pct_within_5_days
FROM warehouse_agg wa
JOIN warehouses w ON w.id = wa.warehouse_id
JOIN tenants t ON t.id = w.tenant_id
ORDER BY wa.orders_shipped DESC NULLS FIRST, w.name ASC NULLS LAST, w.id ASC NULLS LAST
{{fetch_first(50)}}
