SELECT
  t0.invoice_number AS invoice_number,
  t0.status AS status,
  t0.currency_code AS currency_code,
  t0.total AS total,
  t0.amount_due AS amount_due,
  t0.issued_at AS issued_at,
  t0.due_at AS due_at,
  CASE
    WHEN t0.due_at IS NULL THEN 0
    ELSE GREATEST(CAST(DATE '2025-12-31' - CAST(t0.due_at AS date) AS integer), 0)
  END AS days_overdue
FROM invoices t0
JOIN tenants t1 ON t1.id = t0.tenant_id
WHERE t1.name = 'Castell Fasteners'
  AND t0.status IN ('open', 'past_due')
  AND t0.amount_due > 0
ORDER BY t0.amount_due DESC NULLS FIRST, t0.invoice_number ASC NULLS LAST
