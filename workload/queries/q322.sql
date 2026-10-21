select
  t.name as tenant_name,
  ak.name as key_name,
  (date '2026-01-05' - cast(ak.created_at as date)) as age_days,
  ak.last_used_at AS last_used_at,
  (date '2026-01-05' - cast(ak.last_used_at as date)) as days_since_last_use,
  rl.limit_scope AS limit_scope,
  rl.max_requests AS max_requests
from api_keys ak
join tenants t on t.id = ak.tenant_id and t.status = 'churned'
left join rate_limits rl on rl.api_key_id = ak.id
where ak.status = 'active'
order by tenant_name NULLS LAST, key_name NULLS LAST, ak.id NULLS LAST, rl.limit_scope NULLS LAST, rl.id NULLS LAST
