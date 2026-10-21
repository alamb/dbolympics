SELECT
    'segment' AS source,
    t.name AS tenant_name,
    s.name AS name,
    s.is_dynamic AS is_dynamic,
    s.member_count AS member_count,
    substring(cast(s.rules AS varchar), 1, 500) AS rules_definition,
    s.last_refreshed_at AS last_updated
FROM segments s
JOIN tenants t ON t.id = s.tenant_id
UNION ALL
SELECT
    'cohort' AS source,
    t.name AS tenant_name,
    c.name AS name,
    c.is_dynamic AS is_dynamic,
    c.member_count AS member_count,
    substring(cast(c.definition AS varchar), 1, 500) AS rules_definition,
    c.computed_at AS last_updated
FROM cohorts c
JOIN tenants t ON t.id = c.tenant_id
ORDER BY member_count DESC NULLS FIRST, tenant_name ASC NULLS LAST, name ASC NULLS LAST, source NULLS LAST, is_dynamic NULLS LAST, rules_definition NULLS LAST, last_updated NULLS LAST
{{fetch_first(50)}}
