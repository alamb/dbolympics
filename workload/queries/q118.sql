select
  status,
  count(*) as total_requests,
  count(completed_at) as completed_count,
  count(*) - count(completed_at) as not_completed_count,
  min(received_at) as oldest_received_at,
  max(received_at) as newest_received_at,
  avg(cast(case when completed_at is null then {{date_diff_days(received_at, DATE '2026-01-05')}} end as numeric(18,6))) as avg_age_days_not_completed
from data_subject_requests
group by status
order by total_requests desc NULLS FIRST, status asc NULLS LAST
