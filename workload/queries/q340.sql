SELECT
    t2.name AS tenant_name,
    t0.created_at AS created_at,
    t0.subject AS subject,
    t0.status AS status,
    t0.priority AS priority
FROM tickets t0
JOIN users t1 ON t1.id = t0.requester_user_id
JOIN tenants t2 ON t2.id = t1.tenant_id
WHERE t2.name IN (
    'Ampfeld Elektronik',
    'Brennquell Kaffee',
    'Cinder Peak Sports Co.',
    'Cragsley Outfitters',
    'Fennel & Flint',
    'Gratlinie Outdoor',
    'Kibblewick',
    'Marrowby & Vane',
    'Petalcourt Beauty',
    'Studio Vrij'
)
AND t0.created_at >= DATE '2025-10-27'
AND t0.created_at < DATE '2025-11-24'
AND (
    lower(t0.subject) LIKE '%cart%'
    OR lower(t0.subject) LIKE '%remind%'
    OR lower(t0.subject) LIKE '%duplicate%'
    OR lower(t0.subject) LIKE '%missing%'
    OR lower(t0.subject) LIKE '%email%'
)
ORDER BY t0.created_at ASC NULLS LAST, t0.id ASC NULLS LAST
