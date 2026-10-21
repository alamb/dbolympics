select
  t.name as tenant_name,
  count(*) as never_revoked_sessions,
  count(*) filter (where s.expires_at > timestamp '2026-01-05 00:00:00') as still_within_expiry,
  count(*) filter (where s.expires_at <= timestamp '2026-01-05 00:00:00') as already_expired,
  max(s.expires_at) as latest_expires_at,
  max(s.last_seen_at) as latest_last_seen_at
from sessions s
join tenants t on t.id = s.tenant_id
where t.status = 'churned'
  and s.revoked_at is null
group by t.name
order by still_within_expiry desc NULLS FIRST, tenant_name asc NULLS LAST
