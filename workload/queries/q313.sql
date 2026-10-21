SELECT
  t.name AS tenant_name,
  (SELECT COUNT(*) FROM users u WHERE u.tenant_id = t.id AND u.deleted_at IS NULL) AS total_users,
  (SELECT COUNT(*) FROM users u WHERE u.tenant_id = t.id AND u.deleted_at IS NULL AND u.status = 'active') AS active_users,
  (SELECT COUNT(*) FROM sessions s WHERE s.tenant_id = t.id AND s.revoked_at IS NULL) AS open_sessions,
  (SELECT COUNT(*) FROM api_keys ak WHERE ak.tenant_id = t.id AND ak.status = 'active') AS active_api_keys,
  (SELECT COUNT(*) FROM webhooks w WHERE w.tenant_id = t.id AND w.is_active = true) AS active_webhooks,
  (SELECT COUNT(*) FROM service_accounts sa WHERE sa.tenant_id = t.id) AS service_accounts
FROM tenants t
WHERE t.status = 'churned'
ORDER BY active_users DESC NULLS FIRST, tenant_name ASC NULLS LAST, t.id NULLS LAST
