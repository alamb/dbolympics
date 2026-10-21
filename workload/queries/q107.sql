SELECT
  t.name AS tenant_name,
  t.status AS tenant_status,
  COUNT(ak.id) AS total_api_keys,
  SUM(CASE WHEN ak.status = 'active' THEN 1 ELSE 0 END) AS active_api_keys,
  SUM(CASE WHEN ak.status IN ('revoked','expired') THEN 1 ELSE 0 END) AS revoked_or_expired_keys,
  SUM(CASE WHEN ak.status = 'active' AND ak.last_used_at IS NULL THEN 1 ELSE 0 END) AS active_never_used,
  SUM(CASE WHEN ak.status = 'active' AND ak.last_used_at < TIMESTAMP '2025-10-05 00:00:00' THEN 1 ELSE 0 END) AS active_unused_90d,
  MAX(CASE WHEN ak.status = 'active' THEN DATE '2026-01-05' - CAST(ak.created_at AS DATE) ELSE NULL END) AS oldest_active_key_age_days,
  COALESCE(sa.service_account_count, 0) AS service_accounts_count
FROM api_keys ak
JOIN tenants t ON t.id = ak.tenant_id
LEFT JOIN (
  SELECT tenant_id, COUNT(*) AS service_account_count
  FROM service_accounts
  GROUP BY tenant_id
) sa ON sa.tenant_id = t.id
GROUP BY t.id, t.name, t.status, sa.service_account_count
ORDER BY active_never_used DESC NULLS FIRST, active_unused_90d DESC NULLS FIRST, t.name ASC NULLS LAST, t.id NULLS LAST
{{fetch_first(50)}}
