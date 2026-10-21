# Schema

`schema.json` is the source of truth. Everything in `ddl/` is generated from it:

```bash
python3 tools/emit_ddl.py --all                                    # regenerate ddl/*
python3 tools/emit_ddl.py --dialect postgres --with fk,unique,check # opt-in constraints
```

| File | Contents |
|---|---|
| `schema.json` | tables, columns, types, primary keys + metadata (FKs, uniques, checks, defaults, indexes, rules, comments) |
| `ddl/{standard,postgres,duckdb}.sql` | the **basic schema**: column types, `NOT NULL`, `PRIMARY KEY` |
| `ddl/lookup_data.sql` | static rows of the lookup tables (standard `INSERT`) |
| `ddl/rules.sql` | integrity rules standard constraints can't express, as queries returning violating rows |

The basic schema deliberately has no foreign keys, uniques, checks, defaults or indexes.
They stay in `schema.json` as metadata: the data generator must guarantee them, and the
validator checks them. They can be emitted again per engine with `--with`.

## Origin

Derived from heron (commit `859f7c3`, Apache-2.0; its licence and notice are `LICENSE-heron`
and `NOTICE-heron` here). Edit `schema.json`. `NOTICE-heron` is heron's notice as published; the
files it names (`DECISIONS.md`, `docs/...`, `harness/`, `seed/`) are in heron's repository, not
this one.

Added to heron's schema: **`calendar_days`**, a calendar dimension with one row per day,
2023-01-01 to 2026-12-31. Real warehouses have one, and it lets queries build gap-free time
series without engine-specific generators such as `generate_series`.

## Conventions

- **Types**: `bigint`, `integer`, `smallint`, `boolean`, `decimal(p,s)`, `double precision`,
  `varchar(n)`, `date`, `timestamp`, `uuid`, `json`, and standard arrays (`varchar(n) ARRAY`).
  PostgreSQL maps `json` to `jsonb`.
- **Timestamps** are `timestamp` (without time zone) and always hold UTC.
- **Strings** are `varchar(n)` with explicit lengths (heron's `text` and `citext`).
  Lengths are enforced by the generator: DuckDB ignores `n`.
- **Emails** are plain `varchar`; the generator emits them lowercase (no `citext`,
  no collations, whose names are not standardized).
- **IP addresses** are `varchar(45)` (there is no portable 128-bit integer).
- **Enums** became lookup tables (`order_statuses(code, sort_order)` ...) and enum columns
  are `varchar(32)` with a FK to them. `sort_order` keeps PostgreSQL's enum ordering.
- **No namespaces**: heron's 14 Postgres schemas are flattened; `module` in the JSON keeps
  the domain grouping.
- **Ids** are plain `bigint` keys supplied by the generator (no identity columns).
- **Reserved words** (SQL:2023, PostgreSQL, DuckDB; `tools/reserved_words.txt`) are not used
  as identifiers: `returns` -> `order_returns`, `position` -> `sort_position`,
  `value` -> `setting_value`/`discount_value`/..., `interval` -> `billing_interval`, ...
- **Dropped**: `ON DELETE` actions are kept as metadata only; `prices.validity` (a
  `tstzrange` that only backed an `EXCLUDE` constraint) is gone, and the `EXCLUDE` itself
  and the 19 partial unique indexes became rules; `ab_tests.winner_variant_id` lost its FK
  (the schema's only FK cycle) and became a rule.

## JSON format

```jsonc
{
  "modules": [{"name": "sales", "description": "..."}],
  "tables": [{
    "name": "orders", "module": "sales", "kind": "table",       // or "lookup"
    "description": "...", "comment": "...",
    "columns": [{"name": "status", "type": {"name": "varchar", "length": 32},
                 "nullable": false, "default": "'pending'", "comment": "..."}],
    "primary_key": ["id"],
    "foreign_keys": [{"columns": ["user_id"], "references": {"table": "users", "columns": ["id"]},
                      "on_delete": "cascade"}],
    "unique": [{"name": "...", "columns": ["tenant_id", "order_number"]}],
    "checks": [{"name": null, "expr": "grand_total >= 0"}],
    "indexes": [{"name": null, "columns": ["tenant_id"], "where": "..."}],
    "rules": [{"kind": "unique_where", "columns": ["user_id"], "where": "is_default"}],
    "rows": [["pending", 1]]                                       // lookup tables only
  }]
}
```

Type objects: `{"name": "varchar", "length": n}`, `{"name": "decimal", "precision": p, "scale": s}`,
`{"name": "array", "element": <type>}`, or `{"name": <bigint|integer|smallint|boolean|date|timestamp|uuid|json|double>}`.
