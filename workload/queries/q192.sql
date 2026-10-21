select source, row_count, total_abs_quantity, distinct_variants_affected, earliest_date, latest_date from (
  select
    'stock_adjustments table' as source,
    count(*) as row_count,
    sum(abs(quantity_delta)) as total_abs_quantity,
    count(distinct variant_id) as distinct_variants_affected,
    min(created_at) as earliest_date,
    max(created_at) as latest_date,
    0 as sort_order
  from stock_adjustments
  where created_at >= date '2025-01-01' and created_at < date '2026-01-01'

  union all

  select
    'stock_movements kind=adjustment ledger' as source,
    count(*) as row_count,
    sum(abs(quantity)) as total_abs_quantity,
    count(distinct variant_id) as distinct_variants_affected,
    min(occurred_at) as earliest_date,
    max(occurred_at) as latest_date,
    1 as sort_order
  from stock_movements
  where kind = 'adjustment'
    and occurred_at >= date '2025-01-01' and occurred_at < date '2026-01-01'
) t
order by sort_order NULLS LAST
