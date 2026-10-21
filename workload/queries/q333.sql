select
  dsr.public_id as dsar_public_id,
  t.name as tenant_name,
  dsr.subject_user_id AS subject_user_id,
  cast(dsr.completed_at as date) as completed_on,
  u.status as user_status,
  cast(u.deleted_at as date) as user_deleted_at,
  u.updated_at as user_updated_at,
  cast(u.last_login_at as date) as user_last_login_at
from data_subject_requests dsr
join tenants t on t.id = dsr.tenant_id
join users u on u.tenant_id = dsr.tenant_id and u.id = dsr.subject_user_id
where dsr.request_type = 'erasure'
  and dsr.status = 'completed'
order by dsr.completed_at asc NULLS LAST, dsr.public_id asc NULLS LAST
