#!/usr/bin/env python3
"""Generate roleplay/schema.md: the data catalog the role-play agents get (table and column
descriptions, types, keys, status codes). Only facts from schema/schema.json: nothing about
the simulator or the planted stories."""
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
s = json.loads((ROOT / "schema/schema.json").read_text())
tables = {t["name"]: t for t in s["tables"]}

# phrases from the imported schema that would tell the agents this is a benchmark
SCRUB = [(" (~10M rows at bench scale)", ""), ("schemaless jsonb", "schemaless JSON"),
         # DDL file layout, meaningless to readers of the warehouse
         (" but its FK is added in 06_sales.sql to avoid a forward dependency", ""), ("; FK to currencies added in 02_geo.sql", "")]

def clean(text):
    text = " ".join(text.split())
    for a, b in SCRUB:
        text = text.replace(a, b)
    return text

def typ(t):
    n = t["name"]
    if n == "varchar":
        return f"varchar({t['length']})" if t.get("length") else "varchar"
    if n == "decimal":
        return f"decimal({t['precision']},{t['scale']})"
    if n == "array":
        return f"{typ(t['element'])}[]" if t.get("element") else "array"
    return n

out = ["# Tidewick Commerce data warehouse: table catalog", "",
       "The production database, replicated nightly. Every business table carries the merchant's",
       "`tenant_id`. Timestamps are UTC. Money columns are in the row's `currency_code` (merchants sell",
       "in their own currency); `exchange_rates` has daily EUR reference rates.",
       "Status columns reference small lookup tables; their codes are listed with each table.",
       "All tables live in one schema: refer to them by name only (`shipments`, not `sales.shipments`).",
       "The sections below only group the tables by business area.", ""]
by_module = {}
for t in s["tables"]:
    if t.get("kind") == "lookup":
        continue
    by_module.setdefault(t["module"], []).append(t)
for module in sorted(by_module):
    out += [f"## Area: {module}", ""]
    for t in by_module[module]:
        out.append(f"### {t['name']}")
        desc = (t.get("comment") or t.get("description") or "").strip()
        if desc:
            out.append(clean(desc))
        fks = {fk["columns"][0]: fk["references"]["table"] for fk in t.get("foreign_keys", []) if len(fk["columns"]) == 1}
        lines = []
        for c in t["columns"]:
            ref = fks.get(c["name"])
            note = ""
            if ref and tables[ref].get("kind") == "lookup":
                codes = ", ".join(r[0] for r in tables[ref]["rows"])
                note = f" one of: {codes}"
            elif ref:
                note = f" -> {ref}"
            if c.get("comment"):
                note += f" ({clean(c['comment'])})"
            null = "" if c.get("nullable", True) else " not null"
            lines.append(f"- `{c['name']}` {typ(c['type'])}{null}{note}")
        out += lines
        uq = [", ".join(u["columns"]) for u in t.get("unique", [])]
        if uq:
            out.append("- unique: " + "; ".join(f"({u})" for u in uq))
        out.append("")
(ROOT / "roleplay/schema.md").write_text("\n".join(out))
assert "bench" not in "\n".join(out).lower()
print(f"wrote roleplay/schema.md ({sum(len(v) for v in by_module.values())} tables, {len('\n'.join(out))} chars)")
