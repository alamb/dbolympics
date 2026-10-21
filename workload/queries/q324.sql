SELECT
  CASE WHEN dsr.id IS NOT NULL THEN 'Y' ELSE 'N' END AS linked_to_dsar,
  COUNT(*) AS export_count,
  COUNT(DISTINCT de.subject_user_id) AS distinct_subject_users,
  MIN(CAST(de.requested_at AS date)) AS earliest_requested_at,
  MAX(CAST(de.requested_at AS date)) AS latest_requested_at,
  COUNT(*) FILTER (WHERE de.status = 'completed') AS completed_export_count,
  AVG(CAST(de.row_count AS NUMERIC(18,6))) FILTER (WHERE de.row_count IS NOT NULL) AS avg_row_count
FROM data_exports de
LEFT JOIN data_subject_requests dsr ON dsr.data_export_id = de.id
GROUP BY 1
ORDER BY linked_to_dsar DESC NULLS FIRST
