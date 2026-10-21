WITH holds AS (
  SELECT lh.id AS legal_hold_id, lh.tenant_id AS tenant_id, lh.subject_user_id AS subject_user_id, lh.placed_at AS placed_at, t.name AS tenant_name
  FROM legal_holds lh
  JOIN tenants t ON t.id = lh.tenant_id
  WHERE lh.is_active = true
    AND t.name IN ('Marrowby & Vane', 'Rue Lafayette Studio', 'Cragsley Outfitters')
),
latest_payment AS (
  SELECT h.legal_hold_id AS legal_hold_id, p.status AS status, p.updated_at AS updated_at, p.amount AS amount, p.currency_code AS currency_code,
         ROW_NUMBER() OVER (PARTITION BY h.legal_hold_id ORDER BY p.updated_at DESC NULLS FIRST, p.id DESC NULLS FIRST) AS rn
  FROM holds h
  JOIN orders o ON o.user_id = h.subject_user_id AND o.tenant_id = h.tenant_id
  JOIN payments p ON p.order_id = o.id
),
latest_payment_top AS (
  SELECT * FROM latest_payment WHERE rn = 1
),
open_tickets AS (
  SELECT h.legal_hold_id AS legal_hold_id, COUNT(*) AS open_ticket_count
  FROM holds h
  JOIN tickets tk ON tk.requester_user_id = h.subject_user_id AND tk.tenant_id = h.tenant_id
  WHERE tk.status NOT IN ('solved','closed')
  GROUP BY h.legal_hold_id
),
latest_ticket AS (
  SELECT h.legal_hold_id AS legal_hold_id, tk.last_message_at AS last_message_at,
         ROW_NUMBER() OVER (PARTITION BY h.legal_hold_id ORDER BY tk.last_message_at DESC NULLS FIRST) AS rn
  FROM holds h
  JOIN tickets tk ON tk.requester_user_id = h.subject_user_id AND tk.tenant_id = h.tenant_id
),
latest_ticket_top AS (
  SELECT * FROM latest_ticket WHERE rn = 1
)
SELECT
  h.tenant_name AS tenant_name,
  h.subject_user_id AS subject_user_id,
  CAST(h.placed_at AS date) AS hold_placed_date,
  DATE '2026-01-05' - CAST(h.placed_at AS date) AS days_since_placed,
  lp.status AS latest_payment_status,
  CAST(lp.updated_at AS date) AS latest_payment_updated_date,
  lp.amount AS latest_payment_amount,
  lp.currency_code AS latest_payment_currency,
  COALESCE(ot.open_ticket_count, 0) AS open_ticket_count,
  CAST(lt.last_message_at AS date) AS latest_ticket_activity_date,
  DATE '2026-01-05' - CAST(lt.last_message_at AS date) AS days_since_ticket_activity
FROM holds h
LEFT JOIN latest_payment_top lp ON lp.legal_hold_id = h.legal_hold_id
LEFT JOIN open_tickets ot ON ot.legal_hold_id = h.legal_hold_id
LEFT JOIN latest_ticket_top lt ON lt.legal_hold_id = h.legal_hold_id
ORDER BY days_since_placed DESC NULLS FIRST, h.tenant_name NULLS LAST, h.legal_hold_id NULLS LAST
