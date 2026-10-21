#!/usr/bin/env python3
"""Organise the role-play query pool into a catalog (docs/QUERY-GENERATION.md, "Catalog and
traceability"). Nothing is merged or removed: every printed query becomes one entry.

    python3 tools/catalog.py [--db tmp/sf1.duckdb]          # -> workload/catalog.json

For every printed query in the role-play run in runs/ the catalog records:

- its trace key `<run>/<team>/w<week>-n<night>-r<request>` and where its conversation lives
  in the archive (plan, printed reports, memo, engineer thread or transcript);
- a benchmark id `q001`, `q002`, ... in the order the queries were asked (archive order, then the
  order of each run's lineage log, which is chronological);
- a **family**: queries that compute
  the same kind of thing with different predicates or extra columns: the same scanned tables,
  join conditions and aggregate functions in DuckDB's optimised plan (which is independent of
  CTEs, subqueries, aliases and formatting). **Related** queries share tables and joins only;
- **shared measures**: other queries with identical measure columns on the profiling data
  (numeric columns with at least 3 distinct values that are not ids or date parts by name,
  fingerprinted order-independently),
  with the matching column names: the same measure, however differently written;
- a **rewrite class**: queries that match on nearly all their measures (at least 2 shared,
  covering 80% or more of each side's measures) or on their whole result;
- **same text**: identical after collapsing whitespace and case;
- result size and run time on the profiling data (`dbolympics profile`).

Families are numbered in the order their first member was asked.
"""

import argparse
import collections
import datetime
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BIN = ROOT / "target" / "release" / "dbolympics"
RUN_ORDER = ["run6"]
JOINS = ("JOIN", "CROSS_PRODUCT")
AGG = re.compile(r"([a-z_][a-z0-9_]*)\(")


def runs(only=None):
    """The archived runs and their directories: the one in runs/."""
    return [(n, ROOT / "runs") for n in RUN_ORDER if not only or n in only]


def load_queries(only=None):
    out = []
    for run, path in runs(only):
        names = {f"{run}/{p.relative_to(path)}" for p in path.rglob("*") if p.is_file()}
        lin = f"{run}/lineage.jsonl"
        if lin not in names:
            continue
        for line in (path / "lineage.jsonl").read_text().splitlines():
            r = json.loads(line)
            if r.get("verdict") != "ok":
                continue
            t = r.get("tags") or {}
            team, week, night, req = t.get("manager"), t.get("week", 1), t.get("night"), t.get("request")
            key = f"{run}/{team}/w{week}-n{night}-r{req}"
            base = f"{run}/{team}"
            thread = f"{base}/engineers/w{week}-n{night}-r{req}.thread.json"
            transcript = f"{base}/transcripts/engineer-w{week}-n{night}-r{req}.jsonl"
            trace = {
                "archive": "runs", "run": run, "team": team, "week": week, "night": night, "request": req,
                "situation": t.get("situation"), "analyst": t.get("analyst"), "engineer": t.get("engineer"),
                "files": {k: v for k, v in {
                    "manager_note": f"{base}/week{week}-manager-note.md",
                    "plan": f"{base}/week{week}-night{night}-plan.json",
                    "reports": f"{base}/week{week}-night{night}-reports.txt",
                    "memo": f"{base}/week{week}-memo.md",
                    "headlines": f"{base}/week{week}-headlines.json",
                    "engineer_thread": thread if thread in names else None,
                    "engineer_transcript": transcript if transcript in names else None,
                    "analyst_transcript": f"{base}/transcripts/analyst.jsonl",
                    "situation": f"{base}/situation.json",
                }.items() if v and v in names},
            }
            out.append({"key": key, "title": r.get("title"), "question": r.get("question"), "sql": r["sql"],
                        "builds_on": t.get("builds_on") or [], "engineer_note": t.get("engineer_note") or "",
                        "warnings": r.get("warnings") or [], "trace": trace})
    return out


def operators(plan):
    stack = list(plan if isinstance(plan, list) else [plan])
    while stack:
        n = stack.pop()
        if isinstance(n, dict):
            yield n
            stack.extend(n.get("children") or [])


def as_list(v):
    return v if isinstance(v, list) else ([v] if v else [])


def mask(s):
    """Constants out of a filter: strings, numbers, dates and intervals become ?."""
    s = re.sub(r"'(?:[^']|'')*'", "?", s)
    return re.sub(r"(?<![\w#])-?\d+(?:\.\d+)?", "?", s)


def signature(plan):
    """(family key, related key, tables): tables + join conditions + aggregate/window functions,
    and tables + joins alone. Columns and filters are left out."""
    tables, joins, aggs = set(), set(), set()
    for op in operators(plan):
        name, info = op.get("name", ""), op.get("extra_info") or {}
        if "Table" in info:
            tables.add(str(info["Table"]).split(".")[-1])
        elif any(j in name for j in JOINS):
            for c in as_list(info.get("Conditions")):
                joins.add(" ".join(sorted(re.split(r"\s*(?:=|<>|<=|>=|<|>| IS NOT DISTINCT FROM )\s*", c, maxsplit=1))))
        if "Aggregates" in info:
            aggs |= set(AGG.findall(" ".join(as_list(info.get("Aggregates")))))
        if name in ("WINDOW", "STREAMING_WINDOW"):
            aggs |= {f"window:{f}" for f in AGG.findall(json.dumps(info))}
    related = json.dumps([sorted(tables), sorted(joins)])
    return json.dumps([sorted(tables), sorted(joins), sorted(aggs)]), related, sorted(tables)


def letters(i):
    s = ""
    i += 1
    while i:
        i, r = divmod(i - 1, 26)
        s = chr(97 + r) + s
    return s


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", type=Path, default=ROOT / "tmp" / "sf1.duckdb", help="DuckDB file to profile on")
    ap.add_argument("--out", type=Path, default=ROOT / "workload" / "catalog.json")
    ap.add_argument("--runs", default="", help="comma-separated run names (default: all archives)")
    a = ap.parse_args()
    only = [r for r in a.runs.split(",") if r] or None
    qs = load_queries(only)
    print(f"{len(qs)} printed queries in {len(runs(only))} archive(s): {', '.join(n for n, _ in runs(only))}")
    tmp = ROOT / "tmp"
    tmp.mkdir(exist_ok=True)
    (tmp / "catalog-profile-in.jsonl").write_text("".join(json.dumps({"key": q["key"], "sql": q["sql"]}) + "\n" for q in qs))
    subprocess.run([str(BIN), "profile", "--db", str(a.db), "--input", str(tmp / "catalog-profile-in.jsonl"),
                    "--output", str(tmp / "catalog-profile-out.jsonl")], check=True)
    prof = {}
    for line in (tmp / "catalog-profile-out.jsonl").read_text().splitlines():
        r = json.loads(line)
        prof[r["key"]] = r

    # families, related queries, same text, shared measures, rewrites
    fam_of, rel_of, tables_of = {}, {}, {}
    for q in qs:
        p = prof.get(q["key"], {})
        if isinstance(p.get("plan"), (list, dict)):
            fam_of[q["key"]], rel_of[q["key"]], tables_of[q["key"]] = signature(p["plan"])
        else:
            fam_of[q["key"]] = rel_of[q["key"]] = "unplanned:" + q["key"]
            tables_of[q["key"]] = []
    order = []
    for q in qs:
        if fam_of[q["key"]] not in order:
            order.append(fam_of[q["key"]])
    fam_no = {f: i + 1 for i, f in enumerate(order)}
    members = collections.defaultdict(list)
    for q in qs:
        members[fam_of[q["key"]]].append(q["key"])
    norm = lambda s: re.sub(r"\s+", " ", s.strip().rstrip(";")).lower()
    text_groups = collections.defaultdict(list)
    for q in qs:
        text_groups[norm(q["sql"])].append(q["key"])
    related = collections.defaultdict(list)
    for q in qs:
        related[rel_of[q["key"]]].append(q["key"])
    # measures: numeric result columns with at least 3 distinct values
    # (not identifiers or date parts, which every report by merchant or by month shares)
    not_measure = re.compile(r"(^|_)(id|ids|year|years|month|months|quarter|quarters|week|weeks|day|days_of_week|dow|num|yr|mon|qtr|iso_week|rank|rn|row|row_number)$", re.I)
    measures = {q["key"]: {c["hash"]: c["name"] for c in prof.get(q["key"], {}).get("column_fingerprints") or []
                           if c["numeric"] and c["distinct"] >= 3 and not not_measure.search(c["name"].strip().lower())} for q in qs}
    owners = collections.defaultdict(list)
    for k, ms in measures.items():
        for h in ms:
            owners[h].append(k)
    shared = collections.defaultdict(lambda: collections.defaultdict(list))  # a -> b -> [(col a, col b)]
    for h, ks in owners.items():
        for x in ks:
            for y in ks:
                if x != y:
                    shared[x][y].append((measures[x][h], measures[y][h]))
    # rewrite classes: whole result equal, or nearly all measures shared (union-find)
    parent = {q["key"]: q["key"] for q in qs}

    def find(x):
        while parent[x] != x:
            parent[x] = parent[parent[x]]
            x = parent[x]
        return x
    fp_groups = collections.defaultdict(list)
    for q in qs:
        p = prof.get(q["key"], {})
        if p.get("fingerprint") and (p.get("rows") or 0) > 0:
            fp_groups[p["fingerprint"]].append(q["key"])
    for ks in fp_groups.values():
        for k in ks[1:]:
            parent[find(k)] = find(ks[0])
    for x, ys in shared.items():
        for y, cols in ys.items():
            if len(cols) >= 2 and len(cols) >= 0.8 * len(measures[x]) and len(cols) >= 0.8 * len(measures[y]):
                parent[find(x)] = find(y)
    classes = collections.defaultdict(list)
    for q in qs:
        classes[find(q["key"])].append(q["key"])
    rewrite_ids = {}
    for root, ks in classes.items():
        if len(ks) > 1:
            rid = f"r{len({v for v in rewrite_ids.values()}) + 1}"
            for k in ks:
                rewrite_ids[k] = rid

    # ids in the order the queries were asked: per run and week, teams in the order they ran,
    # then night and request (requests of a night run in parallel and finish in any order)
    first = {}
    for i, q in enumerate(qs):
        t = q["trace"]
        first.setdefault((t["run"], t["week"], t["team"]), i)
    run_rank = {n: i for i, (n, _) in enumerate(runs(only))}
    asked = sorted(qs, key=lambda q: (run_rank[q["trace"]["run"]], q["trace"]["week"], first[(q["trace"]["run"], q["trace"]["week"], q["trace"]["team"])],
                                      q["trace"]["night"] or 0, q["trace"]["request"] or 0))
    id_of = {q["key"]: f"q{i + 1:03d}" for i, q in enumerate(asked)}
    qs = asked
    entries = []
    for q in qs:
        k, p = q["key"], prof.get(q["key"], {})
        same_text = [id_of[o] for o in text_groups[norm(q["sql"])] if o != k]
        rw = rewrite_ids.get(k)
        entries.append({
            "id": id_of[k], "family": fam_no[fam_of[k]], "title": q["title"], "question": q["question"], "sql": q["sql"],
            "sources": [{"trace": k, "relation": "original"}],
            "variants": [id_of[o] for o in members[fam_of[k]] if o != k],
            "same_text": same_text,
            "rewrite_class": rw,
            "rewrites": [id_of[o] for o in classes[find(k)] if o != k] if rw else [],
            "related": [id_of[o] for o in related[rel_of[k]] if o != k and fam_of[o] != fam_of[k]],
            "shared_measures": sorted(({"id": id_of[o], "columns": [list(c) for c in cols], "of": len(measures[k])} for o, cols in shared[k].items()),
                                      key=lambda x: -len(x["columns"])),
            "tables": tables_of[k],
            "profile": {"rows": p.get("rows"), "columns": p.get("columns"), "ms": p.get("ms"), "error": p.get("error"), "fingerprint": p.get("fingerprint"),
                        "column_fingerprints": p.get("column_fingerprints")},
            "builds_on": q["builds_on"], "engineer_note": q["engineer_note"], "warnings": q["warnings"],
            "trace": q["trace"],
        })
    commit = subprocess.run(["git", "rev-parse", "HEAD"], capture_output=True, text=True, cwd=ROOT).stdout.strip()
    catalog = {
        "generated_at": datetime.datetime.now().astimezone().isoformat(timespec="seconds"),
        "runs": [n for n, _ in runs(only)],
        "profiled_on": {"db": os.path.relpath(a.db.resolve(), ROOT), "repo_commit": commit},
        "entries": entries,
        "families": [{"family": fam_no[f], "members": [id_of[k] for k in members[f]]} for f in order],
        "forwards": {},  # merged ids -> the id that took over their sources (none yet)
    }
    a.out.parent.mkdir(exist_ok=True)
    a.out.write_text(json.dumps(catalog, indent=1, ensure_ascii=False) + "\n")

    sizes = collections.Counter(len(members[f]) for f in order)
    print(f"wrote {a.out.relative_to(ROOT)}: {len(entries)} entries in {len(order)} families")
    print("family sizes: " + ", ".join(f"{s}: {n}" for s, n in sorted(sizes.items())))
    print(f"related groups (tables+joins) with 2+ queries: {sum(1 for ks in related.values() if len(ks) > 1)} covering {sum(len(ks) for ks in related.values() if len(ks) > 1)} queries")
    print(f"queries sharing a measure with another: {sum(1 for e in entries if e['shared_measures'])}; "
          f"rewrite classes: {len(set(rewrite_ids.values()))} covering {len(rewrite_ids)} queries; same text: {sum(1 for e in entries if e['same_text'])} queries")
    print(f"errors on profiling data: {sum(1 for e in entries if e['profile']['error'])}")


if __name__ == "__main__":
    sys.exit(main())
