SELECT
  tm.title AS macro_title,
  tm.usage_count AS current_usage_count,
  COALESCE(ch.event_count, 0) AS logged_update_event_count,
  ch.sum_increment AS sum_of_logged_increments,
  ch.first_update_at AS first_logged_update_at,
  ch.last_update_at AS last_logged_update_at,
  tm.created_at AS macro_created_at
FROM (
  SELECT id, title, usage_count, created_at
  FROM macros
  ORDER BY usage_count DESC NULLS FIRST, id NULLS LAST
  {{fetch_first(5)}}
) tm
LEFT JOIN (
  SELECT
    entity_id,
    COUNT(*) AS event_count,
    SUM(CAST(JSON_VALUE(after_data, '$.usage_count') AS numeric(18,6)) - CAST(JSON_VALUE(before_data, '$.usage_count') AS numeric(18,6))) AS sum_increment,
    MIN(occurred_at) AS first_update_at,
    MAX(occurred_at) AS last_update_at
  FROM change_history
  WHERE entity_table = 'macros'
    AND {{equals_any('usage_count', changed_columns)}}
  GROUP BY entity_id
) ch ON ch.entity_id = tm.id
ORDER BY tm.usage_count DESC NULLS FIRST, tm.id NULLS LAST
