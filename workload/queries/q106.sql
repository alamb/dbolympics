SELECT
  t.name AS tenant_name,
  t.status AS tenant_status,
  r.name AS role_name,
  u.email AS user_email,
  u.status AS user_status,
  CASE WHEN u.last_login_at IS NULL THEN 'never' ELSE CAST(u.last_login_at AS varchar) END AS last_login,
  CASE WHEN u.last_login_at IS NULL THEN NULL ELSE (DATE '2026-01-05' - CAST(u.last_login_at AS date)) END AS days_since_last_login,
  (DATE '2026-01-05' - CAST(u.created_at AS date)) AS account_age_days,
  COUNT(*) OVER() AS total_qualifying_rows
FROM user_roles ur
JOIN roles r ON r.id = ur.role_id
JOIN users u ON u.id = ur.user_id
JOIN tenants t ON t.id = u.tenant_id
WHERE r.name IN ('Owner','Administrator')
  AND u.deleted_at IS NULL
  AND u.status = 'active'
  AND (u.last_login_at IS NULL OR u.last_login_at < TIMESTAMP '2025-10-05')
ORDER BY u.last_login_at ASC NULLS FIRST, t.name ASC NULLS LAST, u.email ASC NULLS LAST, r.name ASC NULLS LAST, ur.id ASC NULLS LAST
{{fetch_first(50)}}
