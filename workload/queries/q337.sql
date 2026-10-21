select
  t.name as tenant_name,
  jr.queued_at AS queued_at,
  jr.started_at AS started_at,
  jr.finished_at AS finished_at,
  jr.error_message AS error_message,
  jr.attempt AS attempt
from job_runs jr
join jobs j on j.id = jr.job_id
join tenants t on t.id = jr.tenant_id
where j.handler = 'carts.remind'
  and jr.status = 'failed'
  and jr.queued_at >= date '2025-11-01'
  and jr.queued_at < date '2025-12-01'
order by jr.queued_at asc NULLS LAST, t.name asc NULLS LAST, jr.id asc NULLS LAST
