WITH deal_tenants AS (
  SELECT DISTINCT tenant_id FROM deals
),
deal_summary AS (
  SELECT tenant_id,
    {{string_agg_distinct_sorted(currency_code, ',')}} AS deal_currency_codes,
    count(*) AS deal_count,
    count(*) FILTER (WHERE contract_value > 0) AS deals_with_positive_value_count,
    sum(contract_value) AS raw_contract_value_total
  FROM deals
  GROUP BY tenant_id
),
line_items_summary AS (
  SELECT tenant_id,
    sum(quantity * unit_price * (1 - discount_pct)) AS raw_deal_line_items_total
  FROM deal_line_items
  WHERE tenant_id IN (SELECT tenant_id FROM deal_tenants)
  GROUP BY tenant_id
),
orders_summary AS (
  SELECT tenant_id, count(*) AS orders_count_2025
  FROM orders
  WHERE placed_at >= DATE '2025-01-01' AND placed_at < DATE '2026-01-01'
    AND tenant_id IN (SELECT tenant_id FROM deal_tenants)
  GROUP BY tenant_id
),
payments_summary AS (
  SELECT tenant_id, count(*) AS payments_captured_count_2025
  FROM payments
  WHERE status IN ('captured','refunded','partially_refunded')
    AND captured_at >= DATE '2025-01-01' AND captured_at < DATE '2026-01-01'
    AND tenant_id IN (SELECT tenant_id FROM deal_tenants)
  GROUP BY tenant_id
)
SELECT
  t.name AS tenant_name,
  t.default_currency_code AS tenant_default_currency_code,
  ds.deal_currency_codes AS deal_currency_codes,
  ds.deal_count AS deal_count,
  ds.deals_with_positive_value_count AS deals_with_positive_value_count,
  ds.raw_contract_value_total AS raw_contract_value_total,
  li.raw_deal_line_items_total AS raw_deal_line_items_total,
  COALESCE(os.orders_count_2025, 0) AS orders_count_2025,
  COALESCE(ps.payments_captured_count_2025, 0) AS payments_captured_count_2025
FROM deal_summary ds
JOIN tenants t ON t.id = ds.tenant_id
LEFT JOIN line_items_summary li ON li.tenant_id = ds.tenant_id
LEFT JOIN orders_summary os ON os.tenant_id = ds.tenant_id
LEFT JOIN payments_summary ps ON ps.tenant_id = ds.tenant_id
ORDER BY ds.deal_count DESC NULLS FIRST, t.name NULLS LAST, ds.tenant_id NULLS LAST
