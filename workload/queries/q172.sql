WITH wholesale_tenants AS (
  SELECT id AS tenant_id, name AS merchant
  FROM tenants
  WHERE industry ILIKE '%industrial%' OR industry ILIKE '%office equipment%' OR industry ILIKE '%wholesal%'
),
stages AS (
  SELECT
    os.tenant_id AS tenant_id,
    os.id AS stage_id,
    os.name AS stage_name,
    os.sort_position AS sort_position,
    os.win_probability AS win_probability,
    p.name AS pipeline_name
  FROM opportunity_stages os
  JOIN pipelines p ON p.id = os.pipeline_id AND p.tenant_id = os.tenant_id
  JOIN wholesale_tenants wt ON wt.tenant_id = os.tenant_id
),
opp_agg AS (
  SELECT
    o.tenant_id AS tenant_id,
    o.stage_id AS stage_id,
    COUNT(*) FILTER (WHERE o.is_closed = false) AS open_count,
    COUNT(*) FILTER (WHERE o.is_closed = true AND o.is_won = true) AS won_count,
    COUNT(*) FILTER (WHERE o.is_closed = true AND o.is_won = false) AS lost_count,
    AVG(o.amount) AS avg_amount,
    MIN(o.currency_code) AS currency_code
  FROM opportunities o
  JOIN wholesale_tenants wt ON wt.tenant_id = o.tenant_id
  GROUP BY o.tenant_id, o.stage_id
)
SELECT
  wt.merchant AS merchant,
  s.pipeline_name AS pipeline_name,
  s.stage_name AS stage_name,
  s.sort_position AS stage_order,
  s.win_probability AS default_win_probability,
  COALESCE(oa.open_count, 0) AS opportunities_open,
  COALESCE(oa.won_count, 0) AS opportunities_closed_won,
  COALESCE(oa.lost_count, 0) AS opportunities_closed_lost,
  oa.avg_amount AS avg_opportunity_amount,
  oa.currency_code AS currency_code
FROM stages s
JOIN wholesale_tenants wt ON wt.tenant_id = s.tenant_id
LEFT JOIN opp_agg oa ON oa.tenant_id = s.tenant_id AND oa.stage_id = s.stage_id
ORDER BY wt.merchant NULLS LAST, s.sort_position NULLS LAST, s.stage_name NULLS LAST, s.stage_id NULLS LAST
