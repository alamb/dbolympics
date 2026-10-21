WITH role_perm_counts AS (
  SELECT role_id, COUNT(*) AS permission_count
  FROM role_permissions
  GROUP BY role_id
),
role_users AS (
  SELECT ur.role_id AS role_id,
         u.id AS user_id,
         u.status AS status,
         u.last_login_at AS last_login_at
  FROM user_roles ur
  JOIN users u ON u.id = ur.user_id AND u.tenant_id = ur.tenant_id
  WHERE u.deleted_at IS NULL
),
role_user_agg AS (
  SELECT role_id,
         COUNT(*) AS users_assigned,
         COUNT(*) FILTER (WHERE status = 'active') AS users_active,
         COUNT(*) FILTER (WHERE status = 'active' AND (last_login_at IS NULL OR last_login_at < DATE '2025-10-05')) AS users_dormant,
         MAX(last_login_at) AS most_recent_login
  FROM role_users
  GROUP BY role_id
)
SELECT
  t.name AS tenant_name,
  r.name AS role_name,
  r.is_system AS is_system,
  rpc.permission_count AS permission_count,
  rua.users_assigned AS users_assigned,
  rua.users_active AS users_active,
  rua.users_dormant AS users_dormant,
  rua.most_recent_login AS most_recent_login,
  COUNT(*) OVER () AS total_qualifying_roles
FROM roles r
JOIN tenants t ON t.id = r.tenant_id
JOIN role_perm_counts rpc ON rpc.role_id = r.id
JOIN role_user_agg rua ON rua.role_id = r.id
ORDER BY rpc.permission_count DESC NULLS FIRST, rua.users_assigned DESC NULLS FIRST, t.name NULLS LAST, r.name NULLS LAST, r.id NULLS LAST
{{fetch_first(50)}}
