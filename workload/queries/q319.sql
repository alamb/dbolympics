SELECT
    t.name AS tenant_name,
    t.status AS tenant_status,
    ak.name AS key_name,
    (DATE '2026-01-05' - CAST(ak.created_at AS date)) AS age_days,
    ak.last_used_at AS last_used_at,
    CASE WHEN ak.last_used_at IS NULL THEN NULL
         ELSE (DATE '2026-01-05' - CAST(ak.last_used_at AS date))
    END AS days_since_last_use,
    rl.limit_scope AS limit_scope,
    rl.max_requests AS max_requests,
    {{count_distinct_over(ak.id, ())}} AS total_matching_keys
FROM api_keys ak
JOIN tenants t ON t.id = ak.tenant_id
LEFT JOIN rate_limits rl ON rl.api_key_id = ak.id
WHERE ak.status = 'active'
  AND ak.created_at < DATE '2022-01-05'
ORDER BY age_days DESC NULLS FIRST, tenant_name ASC NULLS LAST, key_name ASC NULLS LAST, rl.id ASC NULLS LAST, ak.id ASC NULLS LAST
{{fetch_first(50)}}
