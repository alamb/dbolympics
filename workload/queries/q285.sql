select
  status,
  count(*) as count,
  count(completed_at) as count_with_completed_at,
  min(received_at) as min_received_at,
  max(received_at) as max_received_at
from data_subject_requests
where received_at < date '2023-01-01'
group by status
order by status NULLS LAST
