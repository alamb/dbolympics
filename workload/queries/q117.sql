SELECT
  t.name AS tenant_name,
  rp.name AS policy_name,
  rp.entity_schema AS entity_schema,
  rp.entity_table AS entity_table,
  rp.retention_days AS retention_days,
  rp.action_on_expiry AS action_on_expiry,
  rp.is_active AS is_active,
  rp.last_run_at AS last_run_at
FROM retention_policies rp
JOIN tenants t ON t.id = rp.tenant_id
ORDER BY rp.entity_table NULLS LAST, t.name NULLS LAST, rp.id NULLS LAST
