SELECT
  t.name AS merchant_name,
  t.industry AS industry,
  t.status AS status,
  CAST(o.last_order_ts AS DATE) AS last_order_date,
  (DATE '2026-01-05' - CAST(o.last_order_ts AS DATE)) AS days_since_last_order,
  o.lifetime_orders AS lifetime_orders,
  (SELECT COUNT(*)
     FROM orders o2
    WHERE o2.tenant_id = t.id
      AND o2.status <> 'cancelled'
      AND o2.placed_at > o.last_order_ts - INTERVAL '90 days'
      AND o2.placed_at <= o.last_order_ts
  ) AS orders_90d_before_last,
  (SELECT COUNT(*)
     FROM orders o3
    WHERE o3.tenant_id = t.id
      AND o3.status <> 'cancelled'
      AND o3.placed_at > o.last_order_ts - INTERVAL '180 days'
      AND o3.placed_at <= o.last_order_ts - INTERVAL '90 days'
  ) AS orders_90d_before_prior
FROM tenants t
JOIN (
  SELECT tenant_id, MAX(placed_at) AS last_order_ts, COUNT(*) AS lifetime_orders
  FROM orders
  WHERE status <> 'cancelled'
  GROUP BY tenant_id
  HAVING COUNT(*) >= 20
) o ON o.tenant_id = t.id
WHERE t.status <> 'churned'
ORDER BY days_since_last_order DESC NULLS FIRST, t.id ASC NULLS LAST
{{fetch_first(50)}}
