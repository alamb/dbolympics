SELECT
  status,
  COUNT(*) AS export_count,
  MIN(requested_at) AS earliest_requested_at,
  MAX(requested_at) AS latest_requested_at,
  COUNT(completed_at) AS count_with_completed_at,
  COUNT(file_url) AS count_with_file_url,
  AVG(CAST(row_count AS NUMERIC(18,6))) FILTER (WHERE row_count IS NOT NULL) AS avg_row_count
FROM data_exports
GROUP BY status
ORDER BY export_count DESC NULLS FIRST, status ASC NULLS LAST
