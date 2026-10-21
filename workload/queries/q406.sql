SELECT
  event_name,
  COUNT(*) AS event_count,
  COUNT(DISTINCT user_id) AS distinct_user_count
FROM events
WHERE occurred_at >= TIMESTAMP '2025-01-01'
  AND occurred_at < TIMESTAMP '2026-01-01'
GROUP BY event_name
ORDER BY event_count DESC NULLS FIRST, event_name ASC NULLS LAST
{{fetch_first(30)}}
