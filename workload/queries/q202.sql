with sla_breaches_2025 as (
    select
        breach_kind,
        ticket_id,
        minutes_over,
        breached_at
    from sla_breaches
    where breached_at >= date '2025-01-01'
      and breached_at < date '2026-01-01'
),
breach_kind_summary as (
    select
        breach_kind,
        count(*) as row_count,
        count(distinct ticket_id) as distinct_tickets_affected,
        avg(cast(minutes_over as numeric(18,6))) as avg_minutes_over,
        min(minutes_over) as min_minutes_over,
        max(minutes_over) as max_minutes_over,
        min(breached_at) as earliest_breached_at,
        max(breached_at) as latest_breached_at
    from sla_breaches_2025
    group by breach_kind
)
select
    breach_kind,
    row_count,
    distinct_tickets_affected,
    avg_minutes_over as avg_minutes_over,
    min_minutes_over,
    max_minutes_over,
    earliest_breached_at,
    latest_breached_at
from breach_kind_summary
order by row_count desc NULLS FIRST, breach_kind asc NULLS LAST
