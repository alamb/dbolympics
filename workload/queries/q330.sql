SELECT
  t0.public_id AS dsar_public_id,
  t0.tenant_name AS tenant_name,
  t0.subject_user_id AS subject_user_id,
  t0.completed_at_date AS completed_at_date,
  DATE '2026-01-05' - t0.completed_at_date AS days_since_completed,
  COALESCE(t1.sessions_remaining, 0) AS sessions_remaining,
  t1.most_recent_session_started_at AS most_recent_session_started_at,
  COALESCE(t2.login_attempts_remaining, 0) AS login_attempts_remaining,
  t2.most_recent_login_attempt_at AS most_recent_login_attempt_at,
  COALESCE(t3.access_log_rows_remaining, 0) AS access_log_rows_remaining,
  t3.most_recent_access_log_at AS most_recent_access_log_at
FROM (
  SELECT
    d0.id AS request_id,
    d0.public_id AS public_id,
    d0.tenant_id AS tenant_id,
    tn0.name AS tenant_name,
    d0.subject_user_id AS subject_user_id,
    CAST(d0.completed_at AS date) AS completed_at_date
  FROM data_subject_requests d0
  JOIN tenants tn0 ON tn0.id = d0.tenant_id
  WHERE d0.request_type = 'erasure'
    AND d0.status = 'completed'
    AND d0.subject_user_id IS NOT NULL
) t0
LEFT JOIN (
  SELECT
    s0.tenant_id AS tenant_id,
    s0.user_id AS user_id,
    COUNT(*) AS sessions_remaining,
    MAX(s0.started_at) AS most_recent_session_started_at
  FROM sessions s0
  GROUP BY s0.tenant_id, s0.user_id
) t1 ON t1.tenant_id = t0.tenant_id AND t1.user_id = t0.subject_user_id
LEFT JOIN (
  SELECT
    la0.tenant_id AS tenant_id,
    la0.user_id AS user_id,
    COUNT(*) AS login_attempts_remaining,
    MAX(la0.occurred_at) AS most_recent_login_attempt_at
  FROM login_attempts la0
  WHERE la0.user_id IS NOT NULL
  GROUP BY la0.tenant_id, la0.user_id
) t2 ON t2.tenant_id = t0.tenant_id AND t2.user_id = t0.subject_user_id
LEFT JOIN (
  SELECT
    al0.tenant_id AS tenant_id,
    al0.actor_user_id AS user_id,
    COUNT(*) AS access_log_rows_remaining,
    MAX(al0.occurred_at) AS most_recent_access_log_at
  FROM access_log al0
  WHERE al0.actor_user_id IS NOT NULL
  GROUP BY al0.tenant_id, al0.actor_user_id
) t3 ON t3.tenant_id = t0.tenant_id AND t3.user_id = t0.subject_user_id
ORDER BY t0.completed_at_date ASC NULLS LAST, t0.request_id ASC NULLS LAST
