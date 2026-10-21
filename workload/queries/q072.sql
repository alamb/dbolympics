WITH combined AS (
  SELECT entity_table, action, entity_id, occurred_at
  FROM audit_log
  WHERE occurred_at >= DATE '2023-01-01' AND occurred_at < DATE '2026-01-01'
  UNION ALL
  SELECT entity_table, action, entity_id, occurred_at
  FROM audit_log_archive
  WHERE occurred_at >= DATE '2023-01-01' AND occurred_at < DATE '2026-01-01'
)
SELECT
  entity_table,
  action,
  COUNT(*) AS row_count,
  COUNT(DISTINCT entity_id) AS distinct_entity_id_count,
  MIN(occurred_at) AS min_occurred_at,
  MAX(occurred_at) AS max_occurred_at
FROM combined
WHERE entity_table IN (
  'users','user',
  'roles','role',
  'user_roles','user_role',
  'role_permissions','role_permission',
  'permissions','permission',
  'teams','team',
  'team_members','team_member',
  'api_keys','api_key',
  'invitations','invitation',
  'mfa_devices','mfa_device',
  'sessions','session',
  'service_accounts','service_account',
  'personal_access_tokens','personal_access_token'
)
GROUP BY entity_table, action
ORDER BY entity_table NULLS LAST, action NULLS LAST
