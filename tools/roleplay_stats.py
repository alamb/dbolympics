#!/usr/bin/env python3
"""SQL feature counts for the queries of one or more role-play runs (variety check).

    python3 tools/roleplay_stats.py tmp/roleplay/run1 [tmp/roleplay/run2 ...] [--all] [--per-query]

Reads each run's lineage.jsonl (printed reports only, unless --all), parses every query
with DuckDB's json_serialize_sql and plans it on the empty schema (`<db>.empty.duckdb`, from
run.json) to detect correlated subqueries (DuckDB plans them as DELIM_JOIN). Prints how often
each feature occurs, overall and per engineer persona, plus table coverage. Needs the
`duckdb` command-line tool; opens the database read-only, so it can run during a run.
"""

import argparse
import collections
import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SCRATCH = ROOT / "tmp" / "roleplay-stats-query.sql"


def walk(n):
    """Every JSON object in the parse tree."""
    if isinstance(n, dict):
        yield n
        for v in n.values():
            yield from walk(v)
    elif isinstance(n, list):
        for v in n:
            yield from walk(v)


def correlated(sub):
    """Whether a subquery expression refers to a table alias it does not define itself."""
    inner = list(walk(sub.get("subquery")))
    defined = set()
    for n in inner:
        if n.get("type") in ("BASE_TABLE", "SUBQUERY", "TABLE_FUNCTION") and "class" not in n:
            defined.add((n.get("alias") or n.get("table_name") or "").lower())
        if isinstance(n.get("cte_map"), dict):
            defined |= {e.get("key", "").lower() for e in n["cte_map"].get("map", [])}
    refs = {n["column_names"][0].lower() for n in inner if n.get("class") == "COLUMN_REF" and len(n.get("column_names") or []) >= 2}
    return bool(refs - defined)


def features(sql, empty_db):
    SCRATCH.write_text(sql)
    p = subprocess.run(["duckdb", "-readonly", "-json", str(empty_db), "-c", f"SELECT json_serialize_sql(content) AS j FROM read_text('{SCRATCH}')"],
                       capture_output=True, text=True)
    tree = json.loads(p.stdout)[0]["j"] if p.returncode == 0 and p.stdout.strip() else None
    if isinstance(tree, str):
        tree = json.loads(tree)
    if not tree or tree.get("error"):
        return None
    nodes = list(walk(tree["statements"][0]["node"]))
    f = collections.Counter()
    funcs = {n["function_name"].lower() for n in nodes if isinstance(n.get("function_name"), str)}
    ctes = [e for n in nodes if isinstance(n.get("cte_map"), dict) for e in n["cte_map"].get("map", [])]
    f["CTE"] = bool(ctes)
    f["3+ CTEs"] = len(ctes) >= 3
    f["recursive CTE"] = any(n.get("type") == "RECURSIVE_CTE_NODE" for n in nodes)
    f["window function"] = any(n.get("class") == "WINDOW" for n in nodes)
    f["LAG/LEAD"] = bool(funcs & {"lag", "lead"})
    f["ranking window"] = bool(funcs & {"row_number", "rank", "dense_rank", "ntile", "percent_rank", "cume_dist"})
    subs = [n.get("subquery_type") for n in nodes if n.get("class") == "SUBQUERY"]
    f["scalar subquery"] = "SCALAR" in subs
    f["EXISTS / NOT EXISTS"] = bool({"EXISTS", "NOT_EXISTS"} & set(subs))
    f["IN / ANY subquery"] = "ANY" in subs
    f["derived table"] = any(n.get("type") == "SUBQUERY" and "subquery" in n and n.get("class") is None for n in nodes)
    f["set operation"] = any(n.get("type") == "SET_OPERATION_NODE" for n in nodes)
    joins = [n for n in nodes if n.get("type") == "JOIN"]
    jt = {n.get("join_type") for n in joins}
    f["outer join"] = bool(jt & {"LEFT", "RIGHT", "FULL"})
    f["full outer join"] = "FULL" in jt
    f["semi/anti join"] = bool(jt & {"SEMI", "ANTI"})
    f["5+ joins"] = len(joins) >= 5
    f["GROUP BY"] = any(n.get("group_expressions") for n in nodes)
    f["GROUPING SETS / ROLLUP"] = any(len(n.get("group_sets") or []) > 1 for n in nodes)
    f["HAVING"] = any(n.get("having") for n in nodes if n.get("type") == "SELECT_NODE")
    f["CASE"] = any(n.get("class") == "CASE" for n in nodes)
    f["aggregate FILTER"] = any(n.get("class") == "FUNCTION" and n.get("filter") for n in nodes)
    f["COUNT(DISTINCT)"] = any(n.get("class") == "FUNCTION" and n.get("distinct") for n in nodes)
    f["percentile / median"] = bool(funcs & {"quantile_cont", "quantile_disc", "percentile_cont", "percentile_disc", "median", "quantile"})
    f["string_agg"] = bool(funcs & {"string_agg", "listagg", "group_concat"})
    f["JSON"] = any(x.startswith("json") or x in ("->>", "->") for x in funcs)
    f["UNNEST / arrays"] = bool(funcs & {"unnest", "array_length", "len", "list_contains"}) or any(n.get("type") == "TABLE_FUNCTION" for n in nodes)
    f["date_trunc"] = "date_trunc" in funcs
    f["LIMIT (any level)"] = any(m.get("type") == "LIMIT_MODIFIER" for n in nodes for m in n.get("modifiers") or [] if isinstance(m, dict))
    cte_names = {e.get("key", "").lower() for e in ctes}
    tables = {n.get("table_name", "").lower() for n in nodes if n.get("type") == "BASE_TABLE"} - cte_names
    f["calendar_days"] = "calendar_days" in tables
    # correlated: the planner decorrelates with DELIM_JOIN, but simple EXISTS/IN become plain
    # semi joins, so also look for subqueries that reference an alias defined outside them
    plan = subprocess.run(["duckdb", "-readonly", str(empty_db)], input=f"EXPLAIN {sql};", capture_output=True, text=True).stdout
    f["correlated subquery"] = "DELIM_JOIN" in plan or any(correlated(n) for n in nodes if n.get("class") == "SUBQUERY")
    return f, len(joins), sorted(tables)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("runs", nargs="+", type=Path)
    ap.add_argument("--all", action="store_true", help="include bounced, rejected and failed requests")
    ap.add_argument("--per-query", action="store_true", help="also list the features of every query")
    a = ap.parse_args()

    rows = []
    for run in a.runs:
        db = Path(json.loads((run / "run.json").read_text())["db"])
        empty = db.with_suffix(".empty.duckdb")
        if not empty.exists():
            sys.exit(f"{empty} not found: run `dbolympics load` for {db}")
        for line in (run / "lineage.jsonl").read_text().splitlines():
            r = json.loads(line)
            if r["verdict"] != "ok" and not a.all:
                continue
            got = features(r["sql"], empty)
            if got is None:
                print(f"could not parse: {r['title']}", file=sys.stderr)
                continue
            f, joins, tables = got
            rows.append({"run": run.name, "r": r, "f": f, "joins": joins, "tables": tables})
    SCRATCH.unlink(missing_ok=True)
    if not rows:
        sys.exit("no queries")

    n = len(rows)
    names = list(rows[0]["f"].keys())
    engineers = sorted({x["r"]["tags"].get("engineer", "?") for x in rows})
    by_eng = {e: [x for x in rows if x["r"]["tags"].get("engineer") == e] for e in engineers}

    print(f"{n} queries from {', '.join(sorted({x['run'] for x in rows}))}\n")
    head = f"{'feature':<24} {'all':>10} | " + " ".join(f"{e[:7]:>7}" for e in engineers)
    print(head)
    print(f"{'':<24} {'':>10} | " + " ".join(f"{'n=' + str(len(by_eng[e])):>7}" for e in engineers))
    print("-" * len(head))
    for k in names:
        c = sum(1 for x in rows if x["f"][k])
        per = " ".join(f"{(100 * sum(1 for x in by_eng[e] if x['f'][k]) / len(by_eng[e])):>6.0f}%" for e in engineers)
        print(f"{k:<24} {c:>4} {100 * c / n:>4.0f}% | {per}")

    joins = sorted(x["joins"] for x in rows)
    tabs = sorted(len(x["tables"]) for x in rows)
    print(f"\njoins per query: median {joins[n // 2]}, max {joins[-1]}; tables per query: median {tabs[n // 2]}, max {tabs[-1]}")
    used = collections.Counter(t for x in rows for t in x["tables"])
    print(f"distinct tables touched: {len(used)}")
    print("most used: " + ", ".join(f"{t} {c}" for t, c in used.most_common(15)))
    teams = collections.Counter(x["r"]["tags"].get("manager") for x in rows)
    print("queries per team: " + ", ".join(f"{t} {c}" for t, c in sorted(teams.items())))
    if a.per_query:
        print()
        for x in rows:
            on = [k for k in names if x["f"][k]]
            t = x["r"]["tags"]
            print(f"{x['run']} {t.get('manager')} w{t.get('week')}n{t.get('night')}r{t.get('request')} {t.get('engineer')}: {x['r']['title']}\n    {', '.join(on)}")


if __name__ == "__main__":
    main()
