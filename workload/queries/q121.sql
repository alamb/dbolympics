WITH joined AS (
  SELECT
    dsr.id AS dsr_id,
    dsr.public_id AS dsar_public_id,
    dsr.tenant_id AS dsr_tenant_id,
    dsr.subject_user_id AS dsar_subject_user_id,
    dsr.request_type AS dsar_request_type,
    dsr.received_at AS dsar_received_at,
    lh.id AS lh_id,
    lh.name AS legal_hold_name,
    lh.subject_user_id AS legal_hold_subject_user_id,
    lh.reason AS legal_hold_reason,
    lh.is_active AS legal_hold_is_active,
    lh.placed_at AS legal_hold_placed_at,
    ROW_NUMBER() OVER (
      PARTITION BY dsr.id
      ORDER BY ABS(COALESCE({{date_diff_days(dsr.received_at, lh.placed_at)}}, 999999)) NULLS LAST, lh.id NULLS LAST
    ) AS rn
  FROM data_subject_requests dsr
  LEFT JOIN legal_holds lh
    ON lh.tenant_id = dsr.tenant_id
   AND (
        (dsr.subject_user_id IS NOT NULL AND lh.subject_user_id = dsr.subject_user_id)
        OR
        (dsr.subject_user_id IS NULL AND ABS({{date_diff_days(dsr.received_at, lh.placed_at)}}) <= 3)
   )
  WHERE dsr.status = 'rejected'
)
SELECT
  j.dsar_public_id AS dsar_public_id,
  t.name AS tenant_name,
  j.dsar_subject_user_id AS dsar_subject_user_id,
  j.dsar_request_type AS dsar_request_type,
  CAST(j.dsar_received_at AS DATE) AS dsar_received_on,
  j.legal_hold_name AS legal_hold_name,
  j.legal_hold_subject_user_id AS legal_hold_subject_user_id,
  LEFT(j.legal_hold_reason, 200) AS legal_hold_reason,
  j.legal_hold_is_active AS legal_hold_is_active,
  CAST(j.legal_hold_placed_at AS DATE) AS legal_hold_placed_at,
  {{date_diff_days(j.dsar_received_at, j.legal_hold_placed_at)}} AS days_between,
  CASE
    WHEN j.lh_id IS NULL THEN 'no match'
    WHEN j.dsar_subject_user_id IS NOT NULL THEN 'subject_user_id match'
    ELSE 'tenant + date window fallback'
  END AS join_strategy
FROM joined j
JOIN tenants t ON t.id = j.dsr_tenant_id
WHERE j.rn = 1
ORDER BY j.dsar_received_at ASC NULLS LAST, j.dsar_public_id ASC NULLS LAST
