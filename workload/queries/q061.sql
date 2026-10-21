select
    t.id as tenant_id,
    t.name as a,
    count(o.id) as b
from tenants t
left join orders o on o.tenant_id = t.id
group by t.id, t.name
order by t.id NULLS LAST
