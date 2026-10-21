SELECT t.name AS tenant_name,
       lh.name AS hold_name,
       LEFT(lh.reason, 200) AS reason_truncated,
       lh.entity_table AS entity_table_scope,
       lh.subject_user_id AS subject_user_id,
       lh.is_active AS is_active,
       CAST(lh.placed_at AS date) AS placed_at_date,
       CAST(lh.released_at AS date) AS released_at_date,
       CASE WHEN lh.is_active THEN (DATE '2026-01-05' - CAST(lh.placed_at AS date)) END AS days_since_placed
FROM legal_holds lh
JOIN tenants t ON t.id = lh.tenant_id
ORDER BY lh.is_active DESC NULLS FIRST, lh.placed_at ASC NULLS LAST, lh.id ASC NULLS LAST
