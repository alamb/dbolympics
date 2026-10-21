WITH target_tenants AS (
  SELECT id, name
  FROM tenants
  WHERE name ILIKE '%cragsley%' OR name ILIKE '%marrowby%' OR name ILIKE '%studio vrij%'
),
days AS (
  SELECT calendar_date
  FROM calendar_days
  WHERE calendar_date >= DATE '2025-02-01' AND calendar_date <= DATE '2025-02-28'
),
failed_logins AS (
  SELECT
    la.tenant_id AS tenant_id,
    CAST(la.occurred_at AS DATE) AS login_date,
    COUNT(*) AS failed_count
  FROM login_attempts la
  JOIN target_tenants tt ON tt.id = la.tenant_id
  WHERE la.succeeded = false
    AND la.occurred_at >= TIMESTAMP '2025-02-01'
    AND la.occurred_at < TIMESTAMP '2025-03-01'
  GROUP BY la.tenant_id, CAST(la.occurred_at AS DATE)
),
access_denied_all AS (
  SELECT tenant_id, occurred_at
  FROM audit_log
  WHERE action = 'access_denied'
    AND occurred_at >= TIMESTAMP '2025-02-01'
    AND occurred_at < TIMESTAMP '2025-03-01'
  UNION ALL
  SELECT tenant_id, occurred_at
  FROM audit_log_archive
  WHERE action = 'access_denied'
    AND occurred_at >= TIMESTAMP '2025-02-01'
    AND occurred_at < TIMESTAMP '2025-03-01'
),
access_denied_daily AS (
  SELECT
    CAST(ad.occurred_at AS DATE) AS event_date,
    COUNT(*) AS access_denied_count
  FROM access_denied_all ad
  JOIN target_tenants tt ON tt.id = ad.tenant_id
  GROUP BY CAST(ad.occurred_at AS DATE)
)
SELECT
  d.calendar_date AS date,
  COALESCE(SUM(CASE WHEN tt.name ILIKE '%cragsley%' THEN fl.failed_count END), 0) AS cragsley_failed_logins,
  COALESCE(SUM(CASE WHEN tt.name ILIKE '%marrowby%' THEN fl.failed_count END), 0) AS marrowby_vane_failed_logins,
  COALESCE(SUM(CASE WHEN tt.name ILIKE '%studio vrij%' THEN fl.failed_count END), 0) AS studio_vrij_failed_logins,
  COALESCE(SUM(fl.failed_count), 0) AS total_failed_logins_3_merchants,
  COALESCE(MAX(adn.access_denied_count), 0) AS access_denied_events_3_merchants
FROM days d
LEFT JOIN failed_logins fl ON fl.login_date = d.calendar_date
LEFT JOIN target_tenants tt ON tt.id = fl.tenant_id
LEFT JOIN access_denied_daily adn ON adn.event_date = d.calendar_date
GROUP BY d.calendar_date
ORDER BY d.calendar_date ASC NULLS LAST
