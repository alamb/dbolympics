with month_ends as (
    select
        year_num,
        month_num,
        max(calendar_date) as month_end_date
    from calendar_days
    where year_num between 2023 and 2025
    group by year_num, month_num
),
scoped_tenants as (
    select id as tenant_id, name
    from tenants
    where name in (
        'Ballycorran Industrial Supply',
        'Castell Fasteners',
        'Hanse Industriebedarf',
        'Midland Safety Wear',
        'Polder Packaging'
    )
),
new_business_pipelines as (
    select p.id as pipeline_id, p.tenant_id AS tenant_id
    from pipelines p
    join scoped_tenants st on st.tenant_id = p.tenant_id
    where p.name = 'New business'
),
scoped_opportunities as (
    select o.id AS id, o.tenant_id AS tenant_id, o.created_at AS created_at, o.closed_at AS closed_at
    from opportunities o
    join new_business_pipelines nbp
        on nbp.pipeline_id = o.pipeline_id
        and nbp.tenant_id = o.tenant_id
),
monthly_open_opportunities as (
    select
        me.month_end_date AS month_end_date,
        o.id as opportunity_id,
        {{date_diff_days(CAST(o.created_at AS date), me.month_end_date)}} as age_days
    from month_ends me
    cross join scoped_opportunities o
    where CAST(o.created_at AS date) <= me.month_end_date
        and (o.closed_at is null or CAST(o.closed_at AS date) > me.month_end_date)
)
select
    me.month_end_date AS month_end_date,
    count(moo.opportunity_id) as opportunities_open_at_month_end,
    avg(cast(moo.age_days as numeric(18,6))) as avg_age_days_of_open_opportunities_at_month_end,
    max(moo.age_days) as oldest_age_days_of_open_opportunity_at_month_end
from month_ends me
left join monthly_open_opportunities moo
    on moo.month_end_date = me.month_end_date
group by me.month_end_date
order by me.month_end_date asc NULLS LAST
