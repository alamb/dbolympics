SELECT m.month_label AS month_label,
       (SELECT COUNT(*) FROM tickets t WHERE t.created_at >= m.month_start AND t.created_at < m.month_end) AS ticket_count,
       (SELECT COUNT(*) FROM page_views pv WHERE pv.viewed_at >= m.month_start AND pv.viewed_at < m.month_end AND (pv.path ILIKE '%/help%' OR pv.path ILIKE '%/kb%' OR pv.path ILIKE '%knowledge%' OR pv.path ILIKE '%/faq%' OR pv.path ILIKE '%/support%')) AS kb_engagement_count,
       'page_views.path ILIKE any of %/help%, %/kb%, %knowledge%, %/faq%, %/support% (best-guess pattern)' AS kb_engagement_signal_used
FROM (
  SELECT DISTINCT month_label, month_start, (month_start + INTERVAL '1 month') AS month_end
  FROM calendar_days
  WHERE calendar_date >= DATE '2025-01-01' AND calendar_date <= DATE '2025-12-31'
) m
ORDER BY m.month_label ASC NULLS LAST, m.month_start NULLS LAST
