with flagged_pairs (merchant_name, order_number) as (
  values
    ('OfficeHub Nordic', 'SO-100363'),
    ('Fell & Fjell', '#1179'),
    ('Courtside Collective', 'CC-100015'),
    ('Ampfeld Elektronik', '#1727'),
    ('Ballycorran Industrial Supply', 'SO-100835'),
    ('Ballycorran Industrial Supply', 'SO-100608'),
    ('Midland Safety Wear', 'SO-100307'),
    ('Halden Boots', '#1004'),
    ('Castell Fasteners', 'SO-100233')
),
matched_orders as (
  select
    o.id as order_id,
    o.order_number AS order_number,
    t.name as merchant_name
  from orders o
  join tenants t on t.id = o.tenant_id
  join flagged_pairs fp
    on fp.merchant_name = t.name
   and fp.order_number = o.order_number
),
matched_payments as (
  select
    p.id as payment_id,
    mo.order_id AS order_id,
    mo.order_number AS order_number,
    mo.merchant_name AS merchant_name
  from payments p
  join matched_orders mo on mo.order_id = p.order_id
),
ledger_detail as (
  select
    mp.merchant_name AS merchant_name,
    mp.order_number AS order_number,
    la.name as ledger_account_name,
    la.account_type AS account_type,
    le.direction AS direction,
    le.amount AS amount,
    le.currency_code AS currency_code,
    le.description AS description,
    le.entry_group AS entry_group,
    le.occurred_at AS occurred_at
  from ledger_entries le
  join matched_payments mp on mp.payment_id = le.payment_id
  join ledger_accounts la on la.id = le.ledger_account_id
)
select
  merchant_name,
  order_number,
  ledger_account_name,
  account_type,
  direction,
  amount,
  currency_code,
  description,
  entry_group,
  occurred_at
from ledger_detail
order by merchant_name NULLS LAST, order_number NULLS LAST, occurred_at NULLS LAST, entry_group NULLS LAST, ledger_account_name NULLS LAST, account_type NULLS LAST, direction NULLS LAST, amount NULLS LAST, currency_code NULLS LAST, description NULLS LAST
