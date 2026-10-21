WITH erasures AS (
  SELECT dsr.id AS id, dsr.public_id AS public_id, dsr.tenant_id AS tenant_id, t.name AS tenant_name, dsr.subject_user_id AS subject_user_id, dsr.completed_at AS completed_at
  FROM data_subject_requests dsr
  JOIN tenants t ON t.id = dsr.tenant_id
  WHERE dsr.request_type = 'erasure' AND dsr.status = 'completed'
),
la AS (
  SELECT e.id AS erasure_id,
    count(*) AS login_attempts_count,
    count(*) FILTER (WHERE la.succeeded) AS succeeded_count,
    min(la.occurred_at) AS earliest_attempt,
    max(la.occurred_at) AS latest_attempt
  FROM erasures e
  JOIN login_attempts la
    ON la.tenant_id = e.tenant_id
   AND la.user_id = e.subject_user_id
   AND la.occurred_at > e.completed_at
  GROUP BY e.id
),
se AS (
  SELECT e.id AS erasure_id,
    count(*) AS sessions_count,
    max(s.started_at) AS latest_session_start
  FROM erasures e
  JOIN sessions s
    ON s.tenant_id = e.tenant_id
   AND s.user_id = e.subject_user_id
   AND s.started_at > e.completed_at
  GROUP BY e.id
)
SELECT
  e.public_id AS dsar_public_id,
  e.tenant_name AS tenant_name,
  e.subject_user_id AS subject_user_id,
  CAST(e.completed_at AS date) AS completed_on,
  COALESCE(la.login_attempts_count, 0) AS login_attempts_after,
  COALESCE(la.succeeded_count, 0) AS succeeded_attempts_after,
  la.earliest_attempt AS earliest_login_attempt_after,
  la.latest_attempt AS latest_login_attempt_after,
  COALESCE(se.sessions_count, 0) AS sessions_after,
  se.latest_session_start AS latest_session_start_after
FROM erasures e
LEFT JOIN la ON la.erasure_id = e.id
LEFT JOIN se ON se.erasure_id = e.id
ORDER BY e.completed_at ASC NULLS LAST, e.id ASC NULLS LAST
