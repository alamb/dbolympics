select table_name, year, row_count
from (
  select 'change_history' as table_name, CAST(extract(year from occurred_at) AS int) as year, count(*) as row_count
  from change_history
  group by extract(year from occurred_at)
  union all
  select 'compliance_events' as table_name, CAST(extract(year from occurred_at) AS int) as year, count(*) as row_count
  from compliance_events
  group by extract(year from occurred_at)
  union all
  select 'data_subject_requests' as table_name, CAST(extract(year from received_at) AS int) as year, count(*) as row_count
  from data_subject_requests
  group by extract(year from received_at)
) t
order by table_name NULLS LAST, year NULLS LAST
