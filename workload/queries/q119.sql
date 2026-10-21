SELECT
  t0.entity_schema AS entity_schema,
  t0.entity_table AS entity_table,
  t0.retention_days AS retention_days,
  t0.action_on_expiry AS action_on_expiry,
  COUNT(DISTINCT t0.tenant_id) AS tenant_count,
  COALESCE(SUM(CASE WHEN t0.is_active THEN 1 ELSE 0 END), 0) AS active_count,
  COALESCE(SUM(CASE WHEN t0.last_run_at IS NULL THEN 1 ELSE 0 END), 0) AS never_run_count
FROM retention_policies t0
GROUP BY
  t0.entity_schema,
  t0.entity_table,
  t0.retention_days,
  t0.action_on_expiry
ORDER BY
  t0.entity_table ASC NULLS LAST,
  tenant_count DESC NULLS FIRST,
  t0.entity_schema ASC NULLS LAST,
  t0.retention_days ASC NULLS LAST,
  t0.action_on_expiry ASC NULLS LAST
