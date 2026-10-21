WITH open_requests AS (
    SELECT
        dsr.public_id AS public_id,
        dsr.tenant_id AS tenant_id,
        dsr.request_type AS request_type,
        dsr.status AS status,
        dsr.received_at AS received_at,
        dsr.due_at AS due_at,
        dsr.assigned_to_user_id AS assigned_to_user_id
    FROM data_subject_requests dsr
    WHERE dsr.completed_at IS NULL
),
enriched AS (
    SELECT
        r.public_id AS request_public_id,
        t.name AS tenant_name,
        r.request_type AS request_type,
        r.status AS status,
        CAST(r.received_at AS date) AS received_date,
        CAST(r.due_at AS date) AS due_date,
        DATE '2026-01-05' - CAST(r.received_at AS date) AS days_since_received,
        CASE
            WHEN r.due_at IS NULL THEN NULL
            WHEN DATE '2026-01-05' < CAST(r.due_at AS date) THEN 0
            ELSE DATE '2026-01-05' - CAST(r.due_at AS date)
        END AS days_overdue,
        COALESCE(u.full_name, 'unassigned') AS assigned_to
    FROM open_requests r
    JOIN tenants t ON t.id = r.tenant_id
    LEFT JOIN users u ON u.id = r.assigned_to_user_id
)
SELECT
    request_public_id,
    tenant_name,
    request_type,
    status,
    received_date AS received_at,
    due_date AS due_at,
    days_since_received,
    days_overdue,
    assigned_to
FROM enriched
ORDER BY received_at ASC NULLS LAST, request_public_id ASC NULLS LAST
