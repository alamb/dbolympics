#!/usr/bin/env python3
"""Render a role-play run (tools/roleplay.py --out DIR) as one self-contained web page.

    python3 tools/roleplay_viewer.py tmp/roleplay/run1        # writes tmp/roleplay/run1/index.html

The page shows each track as a thread: the manager's note, every night's request plan, each
request with the engineer's working (messages, dry runs, the submitted SQL) and the printed
report, then the memo; plus a table of all queries. Only the conversation itself is included:
the context Claude Code adds on its own (date, environment, account) is left out.
"""

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def load_jsonl(p):
    return [json.loads(l) for l in p.read_text().splitlines() if l.strip()] if p.exists() else []


def text_of(content):
    if isinstance(content, str):
        return content
    return "\n".join(b.get("text", "") for b in content or [] if isinstance(b, dict) and b.get("type") == "text")


def engineer_thread(transcript):
    """Messages, dry runs and the submit of one engineer session, in order."""
    items, calls = [], {}
    for e in load_jsonl(transcript):
        if e.get("type") not in ("user", "assistant"):
            continue
        content = e["message"]["content"]
        if isinstance(content, str):
            continue  # the request itself
        for b in content:
            t = b.get("type")
            if e["type"] == "assistant" and t == "text" and b.get("text", "").strip():
                items.append({"kind": "text", "text": b["text"].strip()})
            elif t == "tool_use":
                name = b.get("name", "").rsplit("__", 1)[-1]
                item = {"kind": name, "sql": (b.get("input") or {}).get("sql", ""), "note": (b.get("input") or {}).get("note", "")}
                calls[b.get("id")] = item
                items.append(item)
            elif t == "tool_result" and b.get("tool_use_id") in calls:
                calls[b["tool_use_id"]]["result"] = text_of(b.get("content")).strip()
    return items


def split_reports(text):
    """{index: printed report} from a night's reports file."""
    out = {}
    for m in re.finditer(r"=== Report (\d+): .*? ===\n(.*?)(?=\n=== Report \d+: |\Z)", text, re.S):
        # the title, question and engineer's note are shown separately
        lines = [l for l in m.group(2).strip().splitlines() if not l.startswith(("Engineer's note:", "REPORT:", "QUESTION:"))]
        out[int(m.group(1))] = "\n".join(lines).strip()
    return out


def analyst_prompts(transcript):
    """The analyst's incoming messages (string prompts) in order."""
    return [e["message"]["content"] for e in load_jsonl(transcript)
            if e.get("type") == "user" and isinstance(e["message"]["content"], str)]


def build(run):
    meta = json.loads((run / "run.json").read_text()) if (run / "run.json").exists() else {}
    usage = json.loads((run / "usage.json").read_text()) if (run / "usage.json").exists() else {}
    pfile = run / "personas.json" if (run / "personas.json").exists() else ROOT / "roleplay" / "personas.json"
    personas = json.loads(pfile.read_text())
    lineage = load_jsonl(run / "lineage.jsonl")
    dry = load_jsonl(run / "dry_runs.jsonl")

    tracks = []
    for m in personas["managers"]:
        tdir = run / m["id"]
        if not tdir.is_dir():
            continue
        lin = [r for r in lineage if r.get("tags", {}).get("manager") == m["id"]]
        analyst_name = next((r["tags"].get("analyst") for r in lin), None)
        analyst = next((a for a in personas["analysts"] if a["name"] == analyst_name), {"name": analyst_name or "analyst", "style": ""})
        prompts = analyst_prompts(tdir / "transcripts" / "analyst.jsonl")
        weeks = []
        w = 1
        while (tdir / f"week{w}-manager-note.md").exists():
            nights = []
            n = 1
            while (tdir / f"week{w}-night{n}-plan.json").exists():
                plan = json.loads((tdir / f"week{w}-night{n}-plan.json").read_text())
                rep_file = tdir / f"week{w}-night{n}-reports.txt"
                if rep_file.exists():
                    reports = split_reports(rep_file.read_text())
                else:  # older runs: the reports are in the analyst's next prompt
                    marker = f"The printed reports of night {n}:"
                    hits = [p for p in prompts if p.startswith(marker)]
                    reports = split_reports(hits[w - 1]) if len(hits) >= w else {}
                reqs = []
                for i, req in enumerate(plan.get("requests") or [], 1):
                    key = f"w{w}-n{n}-r{i}"
                    run_row = next((r for r in lin if r["tags"].get("week") == w and r["tags"].get("night") == n and r["tags"].get("request") == i), None)
                    tfile = tdir / "engineers" / f"{key}.thread.json"
                    # the backend-neutral thread the orchestrator records; older runs: the transcript
                    thread = json.loads(tfile.read_text()) if tfile.exists() else engineer_thread(tdir / "transcripts" / f"engineer-{key}.jsonl")
                    if not thread:  # no transcript (mock runs): rebuild from the logs
                        for d in dry:
                            t = d.get("tags", {})
                            if t.get("manager") == m["id"] and (t.get("week"), t.get("night"), t.get("request")) == (w, n, i):
                                thread.append({"kind": "dry_run", "sql": d["sql"], "result": d["verdict"].upper() + "".join("\n  - " + x for x in d["reasons"])})
                        sub = tdir / "engineers" / f"{key}.submit.json"
                        if sub.exists():
                            s = json.loads(sub.read_text())
                            thread.append({"kind": "submit", "sql": s.get("sql", ""), "note": s.get("note", ""), "result": "Submitted."})
                    reqs.append({
                        "key": key, "request": req, "thread": thread, "report": reports.get(i, ""),
                        "engineer": (run_row or {}).get("tags", {}).get("engineer") or next((d["tags"]["engineer"] for d in dry if d.get("tags", {}).get("manager") == m["id"] and (d["tags"].get("week"), d["tags"].get("night"), d["tags"].get("request")) == (w, n, i)), ""),
                        "verdict": (run_row or {}).get("verdict", "missing"), "rows": (run_row or {}).get("rows"),
                        "elapsed_ms": (run_row or {}).get("elapsed_ms"), "note": (run_row or {}).get("tags", {}).get("engineer_note", ""),
                        "warnings": (run_row or {}).get("warnings", []),
                    })
                nights.append({"night": n, "notes": plan.get("notes", ""), "requests": reqs})
                n += 1
            memo = tdir / f"week{w}-memo.md"
            mgrs = {x["id"]: x for x in personas["managers"]}
            coowners = [{"id": f.stem.split("coowner-")[1], "text": f.read_text(), **{k: mgrs.get(f.stem.split("coowner-")[1], {}).get(k, "") for k in ("name", "title")}}
                        for f in sorted(tdir.glob(f"week{w}-coowner-*.md"))]
            hf = tdir / f"week{w}-headlines.json"
            weeks.append({"week": w, "note": (tdir / f"week{w}-manager-note.md").read_text(), "nights": nights,
                          "memo": memo.read_text() if memo.exists() else "", "coowners": coowners,
                          "headlines": json.loads(hf.read_text()) if hf.exists() else []})
            w += 1
        sit = json.loads((tdir / "situation.json").read_text()) if (tdir / "situation.json").exists() else None
        tracks.append({"id": m["id"], "manager": m, "analyst": analyst, "weeks": weeks, "situation": sit})

    queries = [{
        "manager": r["tags"].get("manager"), "week": r["tags"].get("week"), "night": r["tags"].get("night"),
        "request": r["tags"].get("request"), "title": r.get("title"), "question": r.get("question"),
        "engineer": r["tags"].get("engineer"), "verdict": r.get("verdict"), "reasons": r.get("reasons", []),
        "warnings": r.get("warnings", []),
        "rows": r.get("rows"), "elapsed_ms": r.get("elapsed_ms"), "tables": r.get("tables", []),
        "dry_runs": r["tags"].get("dry_runs"), "sql": r.get("sql"), "builds_on": r["tags"].get("builds_on", []),
    } for r in lineage]
    channels = [{"week": int(f.stem[4:].split("-")[0]), "text": f.read_text()} for f in sorted(run.glob("week*-channel.md"))]
    return {"run": run.name, "meta": meta, "usage": usage, "tracks": tracks, "queries": queries, "channels": channels}


PAGE = r"""<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>Tidewick Analytics Week</title>
<link rel="preconnect" href="https://fonts.googleapis.com">
<link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&family=JetBrains+Mono:wght@400;500&display=swap" rel="stylesheet">
<script src="https://cdnjs.cloudflare.com/ajax/libs/marked/12.0.2/marked.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/highlight.min.js"></script>
<script src="https://cdnjs.cloudflare.com/ajax/libs/highlight.js/11.9.0/languages/sql.min.js"></script>
<style>
:root {
  --bg: #f6f5f1; --panel: #ffffff; --ink: #1d232b; --muted: #66707c; --line: #e3e0d8;
  --accent: #0f6e6e; --accent-soft: #e2f0ef; --mgr: #8a4b14; --mgr-soft: #f8ecdf;
  --an: #2f4fa3; --an-soft: #e7ecf9; --eng: #5b3c99; --eng-soft: #efe9f8;
  --ok: #1f7a3d; --ok-soft: #e3f3e7; --bad: #a3302a; --bad-soft: #f8e4e2; --warn: #8a6a00; --warn-soft: #f7f0d6;
  --code: #f3f1ec; --shadow: 0 1px 2px rgba(0,0,0,.05), 0 2px 10px rgba(0,0,0,.04);
}
@media (prefers-color-scheme: dark) { :root:not([data-theme="light"]) {
  --bg: #14171b; --panel: #1c2026; --ink: #e6e8eb; --muted: #9aa3ad; --line: #2c323a;
  --accent: #4fb8b4; --accent-soft: #173331; --mgr: #e0a56a; --mgr-soft: #332518;
  --an: #8ea9f0; --an-soft: #1d2640; --eng: #b99af0; --eng-soft: #2a2140;
  --ok: #6cc98a; --ok-soft: #173322; --bad: #ef8a83; --bad-soft: #3a1d1b; --warn: #e0c35a; --warn-soft: #332c12;
  --code: #161a1f; --shadow: none;
}}
:root[data-theme="dark"] {
  --bg: #14171b; --panel: #1c2026; --ink: #e6e8eb; --muted: #9aa3ad; --line: #2c323a;
  --accent: #4fb8b4; --accent-soft: #173331; --mgr: #e0a56a; --mgr-soft: #332518;
  --an: #8ea9f0; --an-soft: #1d2640; --eng: #b99af0; --eng-soft: #2a2140;
  --ok: #6cc98a; --ok-soft: #173322; --bad: #ef8a83; --bad-soft: #3a1d1b; --warn: #e0c35a; --warn-soft: #332c12;
  --code: #161a1f; --shadow: none;
}
* { box-sizing: border-box; }
body { margin: 0; background: var(--bg); color: var(--ink); font: 15px/1.55 Inter, system-ui, sans-serif; }
code, pre, .mono { font-family: "JetBrains Mono", ui-monospace, monospace; font-size: 12.5px; font-variant-ligatures: none; }
a { color: var(--accent); }
.layout { display: grid; grid-template-columns: 270px 1fr; min-height: 100vh; }
nav { border-right: 1px solid var(--line); padding: 20px 14px; position: sticky; top: 0; height: 100vh; overflow-y: auto; background: var(--panel); }
nav h1 { font-size: 16px; margin: 0 0 2px; }
nav .sub { color: var(--muted); font-size: 12.5px; margin-bottom: 18px; }
nav a.item { display: block; padding: 8px 10px; border-radius: 8px; text-decoration: none; color: var(--ink); margin-bottom: 2px; }
nav a.item:hover { background: var(--bg); }
nav a.item.active { background: var(--accent-soft); }
nav a.item .t { font-weight: 600; font-size: 14px; }
nav a.item .s { color: var(--muted); font-size: 12px; }
nav .group { text-transform: uppercase; letter-spacing: .06em; font-size: 11px; color: var(--muted); margin: 18px 10px 6px; }
main { padding: 28px clamp(16px, 4vw, 48px) 80px; max-width: 1100px; min-width: 0; }
.stats { display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 10px; margin: 16px 0 26px; }
.stat { background: var(--panel); border: 1px solid var(--line); border-radius: 10px; padding: 12px 14px; box-shadow: var(--shadow); }
.stat .v { font-size: 22px; font-weight: 700; }
.stat .k { color: var(--muted); font-size: 12px; }
h2 { font-size: 22px; margin: 0 0 4px; }
h3.week { margin: 34px 0 10px; font-size: 13px; text-transform: uppercase; letter-spacing: .07em; color: var(--muted); }
.night { margin: 26px 0 8px; display: flex; align-items: center; gap: 10px; }
.night span { font-weight: 700; font-size: 14px; }
.night hr { flex: 1; border: 0; border-top: 1px dashed var(--line); }
.card { background: var(--panel); border: 1px solid var(--line); border-radius: 12px; padding: 16px 18px; margin: 12px 0; box-shadow: var(--shadow); }
.who { display: flex; align-items: center; gap: 10px; margin-bottom: 8px; }
.avatar { width: 32px; height: 32px; border-radius: 50%; display: grid; place-items: center; font-weight: 700; font-size: 12px; flex: none; }
.mgr .avatar { background: var(--mgr-soft); color: var(--mgr); } .an .avatar { background: var(--an-soft); color: var(--an); } .eng .avatar { background: var(--eng-soft); color: var(--eng); }
.who .n { font-weight: 600; } .who .r { color: var(--muted); font-size: 12.5px; }
.md > :first-child { margin-top: 0; } .md > :last-child { margin-bottom: 0; }
.md h1, .md h2, .md h3 { font-size: 16px; margin: 16px 0 6px; }
.md table { border-collapse: collapse; } .md td, .md th { border: 1px solid var(--line); padding: 3px 8px; }
.req { border-left: 3px solid var(--an); }
.req .title { font-weight: 600; font-size: 15px; }
.req .q { margin: 4px 0 8px; }
.meta { color: var(--muted); font-size: 13px; }
.meta b { color: var(--ink); font-weight: 500; }
ul.cols { margin: 6px 0; padding-left: 18px; font-size: 13.5px; }
details { margin-top: 10px; }
details > summary { cursor: pointer; color: var(--accent); font-size: 13.5px; font-weight: 500; list-style: none; }
details > summary::before { content: "▸ "; } details[open] > summary::before { content: "▾ "; }
.badge { display: inline-block; font-size: 11.5px; font-weight: 600; padding: 2px 8px; border-radius: 99px; text-transform: uppercase; letter-spacing: .04em; vertical-align: middle; }
.b-ok, .b-compiles, .b-submit { background: var(--ok-soft); color: var(--ok); }
.b-bounced, .b-warning { background: var(--warn-soft); color: var(--warn); }
.warn { background: var(--warn-soft); color: var(--warn); border-radius: 8px; padding: 6px 10px; font-size: 13px; margin: 6px 0; }
.b-rejected, .b-failed, .b-missing { background: var(--bad-soft); color: var(--bad); }
.thread { margin-top: 10px; border-left: 2px solid var(--eng-soft); padding-left: 14px; }
.step { margin: 10px 0; }
.step .lbl { font-size: 12.5px; color: var(--muted); margin-bottom: 4px; }
pre { background: var(--code); border: 1px solid var(--line); border-radius: 8px; padding: 10px 12px; overflow-x: auto; margin: 6px 0; white-space: pre; }
pre.result { white-space: pre-wrap; }
pre code.hljs { background: transparent; padding: 0; }
.report pre { font-size: 12px; line-height: 1.45; }
.report .head { display: flex; align-items: center; gap: 8px; flex-wrap: wrap; margin-top: 12px; }
.note { background: var(--eng-soft); border-radius: 8px; padding: 8px 12px; font-size: 13.5px; margin: 8px 0; }
.memo { border-left: 3px solid var(--an); }
.mgrcard { border-left: 3px solid var(--mgr); }
table.q { width: 100%; border-collapse: collapse; background: var(--panel); border: 1px solid var(--line); border-radius: 10px; overflow: hidden; font-size: 13.5px; }
table.q th { text-align: left; font-weight: 600; font-size: 12px; color: var(--muted); padding: 8px 10px; border-bottom: 1px solid var(--line); cursor: pointer; user-select: none; white-space: nowrap; }
table.q td { padding: 8px 10px; border-bottom: 1px solid var(--line); vertical-align: top; }
table.q tr.sqlrow td { background: var(--bg); }
table.q tr.main:hover td { background: var(--accent-soft); cursor: pointer; }
.num { text-align: right; font-variant-numeric: tabular-nums; }
.toolbar { display: flex; gap: 8px; flex-wrap: wrap; margin: 14px 0; }
.toolbar input, .toolbar select { font: inherit; font-size: 13.5px; padding: 6px 10px; border: 1px solid var(--line); border-radius: 8px; background: var(--panel); color: var(--ink); }
.tablewrap { overflow-x: auto; }
.theme { margin-top: 18px; font: inherit; font-size: 12.5px; padding: 5px 10px; border-radius: 8px; border: 1px solid var(--line); background: var(--panel); color: var(--ink); cursor: pointer; }
@media (max-width: 820px) {
  .layout { grid-template-columns: 1fr; }
  nav { position: static; height: auto; border-right: 0; border-bottom: 1px solid var(--line); }
}
</style>
</head>
<body>
<div class="layout">
  <nav id="nav"></nav>
  <main id="main"></main>
</div>
<script id="data" type="application/json">__DATA__</script>
<script>
const D = JSON.parse(document.getElementById('data').textContent);
const $ = (s, el = document) => el.querySelector(s);
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const md = s => window.marked ? marked.parse(s || '') : `<p style="white-space:pre-wrap">${esc(s)}</p>`;
const initials = n => (n || '?').split(/\s+/).map(w => w[0]).slice(0, 2).join('').toUpperCase();
const sql = s => `<pre><code class="language-sql">${esc(s)}</code></pre>`;
const badge = v => `<span class="badge b-${esc((v || '').toLowerCase())}">${esc(v)}</span>`;
const fmtMs = ms => ms == null ? '' : ms < 1000 ? `${ms} ms` : `${(ms / 1000).toFixed(1)} s`;

try { const t = localStorage.getItem('rp-theme'); if (t) document.documentElement.dataset.theme = t; } catch (e) {}
function toggleTheme() {
  const cur = document.documentElement.dataset.theme || (matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light');
  const next = cur === 'dark' ? 'light' : 'dark';
  document.documentElement.dataset.theme = next;
  try { localStorage.setItem('rp-theme', next); } catch (e) {}
}

function statusOf(report) {
  const m = /STATUS: (\w+)/.exec(report || '');
  return m ? m[1] : (report ? 'OK' : '');
}

function nav(active) {
  const verdicts = {};
  D.queries.forEach(q => verdicts[q.verdict] = (verdicts[q.verdict] || 0) + 1);
  let h = `<h1>Tidewick Analytics</h1><div class="sub">Week of Monday 2026-01-05 · run <span class="mono">${esc(D.run)}</span></div>`;
  h += `<a class="item ${active === 'overview' ? 'active' : ''}" href="#overview"><div class="t">Overview</div><div class="s">${D.tracks.length} teams · ${D.queries.length} reports</div></a>`;
  h += `<a class="item ${active === 'queries' ? 'active' : ''}" href="#queries"><div class="t">All queries</div><div class="s">${Object.entries(verdicts).map(([k, v]) => `${v} ${k}`).join(' · ') || 'none yet'}</div></a>`;
  h += `<div class="group">Leaders and their analysts</div>`;
  for (const t of D.tracks) {
    const n = t.weeks.reduce((a, w) => a + w.nights.reduce((b, x) => b + x.requests.length, 0), 0);
    h += `<a class="item ${active === t.id ? 'active' : ''}" href="#track/${t.id}"><div class="t">${esc(t.manager.title)}</div><div class="s">${esc(t.manager.name)} → ${esc(t.analyst.name)} · ${n} reports${t.manager.reports_to ? ' · reports to ' + esc(t.manager.reports_to) : ''}</div></a>`;
  }
  h += `<button class="theme" onclick="toggleTheme()">Light / dark</button>`;
  $('#nav').innerHTML = h;
}

function overview() {
  const u = D.usage || {}, tok = u.tokens || {};
  const v = {}; D.queries.forEach(q => v[q.verdict] = (v[q.verdict] || 0) + 1);
  const mins = u.seconds ? `${Math.floor(u.seconds / 60)}m ${Math.round(u.seconds % 60)}s` : '–';
  let h = `<h2>The week at a glance</h2>
    <div class="meta">Each leader sent a note to their analyst; analysts requested reports each night; data engineers wrote the SQL; the warehouse printed the reports; analysts wrote a memo.</div>
    <div class="stats">
      <div class="stat"><div class="v">${D.tracks.length}</div><div class="k">leader / analyst teams</div></div>
      <div class="stat"><div class="v">${D.queries.length}</div><div class="k">reports submitted</div></div>
      <div class="stat"><div class="v">${v.ok || 0}</div><div class="k">printed</div></div>
      <div class="stat"><div class="v">${(v.bounced || 0) + (v.rejected || 0) + (v.failed || 0)}</div><div class="k">bounced / rejected / failed</div></div>
      <div class="stat"><div class="v">${D.queries.filter(q => (q.warnings || []).length).length}</div><div class="k">with bundling warnings</div></div>
      <div class="stat"><div class="v">${u.calls ?? u.cli_calls ?? '–'}</div><div class="k">agent calls (${u.turns ?? '–'} turns)</div></div>
      <div class="stat"><div class="v">${mins}</div><div class="k">wall clock</div></div>
      <div class="stat"><div class="v">${u.api_equivalent_usd != null ? '$' + u.api_equivalent_usd : '–'}</div><div class="k">API-equivalent cost</div></div>
      <div class="stat"><div class="v">${((tok.cache_read_input_tokens || 0) / 1e6).toFixed(1)}M</div><div class="k">cached input tokens read</div></div>
    </div>`;
  for (const c of (D.channels || [])) h += `<div class="card"><details><summary>Analytics channel, week ${c.week}</summary><div class="md">${md(c.text)}</div></details></div>`;
  for (const t of D.tracks) {
    const last = t.weeks[t.weeks.length - 1];
    h += `<div class="card memo"><div class="who an"><div class="avatar">${initials(t.analyst.name)}</div><div><div class="n">${esc(t.analyst.name)} → ${esc(t.manager.name)}, ${esc(t.manager.title)}</div><div class="r">Memo, week ${last ? last.week : 1} · <a href="#track/${t.id}">open the thread</a></div></div></div>
      <details><summary>Read the memo</summary><div class="md">${md(last ? last.memo : '')}</div></details></div>`;
  }
  return h;
}

function engineerThread(r) {
  let h = '<div class="thread">';
  let dry = 0;
  for (const s of r.thread) {
    if (s.kind === 'text') h += `<div class="step"><div class="md">${md(s.text)}</div></div>`;
    else if (s.kind === 'dry_run') {
      dry++;
      const st = statusOf(s.result) || (s.result || '').split(/\s/)[0];
      h += `<div class="step"><div class="lbl">Dry run ${dry} ${badge(st)}</div><details><summary>SQL</summary>${sql(s.sql)}</details><pre class="result">${esc(s.result || '')}</pre></div>`;
    } else if (s.kind === 'submit') {
      h += `<div class="step"><div class="lbl">Submitted ${badge('submit')}</div>${sql(s.sql)}${s.note ? `<div class="note"><b>Note to the analyst:</b> ${esc(s.note)}</div>` : ''}</div>`;
    }
  }
  return h + '</div>';
}

function track(id) {
  const t = D.tracks.find(x => x.id === id);
  if (!t) return '<p>Unknown track.</p>';
  let h = `<h2>${esc(t.manager.title)}</h2>
    <div class="meta">${esc(t.manager.name)} · ${esc(t.manager.background)}<br>Analyst <b>${esc(t.analyst.name)}</b>, ${esc(t.analyst.style)}</div>
    ${t.situation ? `<div class="card"><div class="meta"><b>Situation${t.situation.week > 1 ? ' (from week ' + t.situation.week + ')' : ''}</b> · ${esc(t.situation.id)}${t.situation.with.length ? ' · with ' + t.situation.with.map(esc).join(', ') : ''}</div><div>${esc(t.situation.text)}</div></div>` : ''}`;
  for (const w of t.weeks) {
    h += `<h3 class="week">Week ${w.week}</h3>`;
    h += `<div class="card mgrcard"><div class="who mgr"><div class="avatar">${initials(t.manager.name)}</div><div><div class="n">${esc(t.manager.name)}</div><div class="r">${esc(t.manager.title)} → ${esc(t.analyst.name)} · Monday</div></div></div><div class="md">${md(w.note)}</div></div>`;
    for (const c of (w.coowners || [])) h += `<div class="card mgrcard"><div class="who mgr"><div class="avatar">${initials(c.name)}</div><div><div class="n">${esc(c.name)}</div><div class="r">${esc(c.title)}, co-owner → ${esc(t.analyst.name)}</div></div></div><div class="md">${md(c.text)}</div></div>`;
    for (const n of w.nights) {
      h += `<div class="night"><span>Night ${n.night}</span><hr></div>`;
      if (n.notes) h += `<div class="card"><div class="who an"><div class="avatar">${initials(t.analyst.name)}</div><div><div class="n">${esc(t.analyst.name)}</div><div class="r">Planning notes</div></div></div><div class="md">${md(n.notes)}</div></div>`;
      if (!n.requests.length) h += `<div class="meta">No reports requested.</div>`;
      for (const r of n.requests) {
        const q = r.request, st = statusOf(r.report) || r.verdict.toUpperCase();
        h += `<div class="card req" id="${esc(r.key)}">
          <div class="title">${esc(q.title)} ${badge(st)}${(r.warnings || []).length ? ' ' + badge('warning') : ''}</div>
          ${(r.warnings || []).map(w => `<div class="warn">Warning (not shown to the agents): ${esc(w)}</div>`).join('')}
          <div class="q">${esc(q.question)}</div>
          <div class="meta">Grain: <b>${esc(q.grain)}</b>${r.rows != null ? ` · ${r.rows} rows` : ''}${r.elapsed_ms != null ? ` · ran in ${fmtMs(r.elapsed_ms)}` : ''} · written by <b>${esc(r.engineer)}</b>${(q.builds_on || []).length ? ` · follows up on: ${q.builds_on.map(esc).join('; ')}` : ''}</div>
          <details><summary>Request details</summary><ul class="cols">${(q.columns || []).map(c => `<li>${esc(c)}</li>`).join('')}</ul><div class="meta">${esc(q.details)}</div></details>
          <details><summary>How ${esc(r.engineer)} wrote it (${r.thread.filter(s => s.kind === 'dry_run').length} dry runs)</summary>${engineerThread(r)}</details>
          <div class="report">${r.note ? `<div class="note"><b>Engineer's note:</b> ${esc(r.note)}</div>` : ''}<pre>${esc(r.report || '(no report)')}</pre></div>
        </div>`;
      }
    }
    h += `<div class="night"><span>Friday</span><hr></div>`;
    h += `<div class="card memo"><div class="who an"><div class="avatar">${initials(t.analyst.name)}</div><div><div class="n">${esc(t.analyst.name)}</div><div class="r">Memo to ${esc(t.manager.name)}</div></div></div><div class="md">${md(w.memo)}</div></div>`;
    if ((w.headlines || []).length) h += `<div class="card"><div class="meta"><b>Posted to the analytics channel</b></div><ul class="cols">${w.headlines.map(x => `<li>${esc(x)}</li>`).join('')}</ul></div>`;
  }
  return h;
}

let sortKey = null, sortDir = 1;
function queries() {
  const mgrs = [...new Set(D.queries.map(q => q.manager))];
  const verdicts = [...new Set(D.queries.map(q => q.verdict))];
  return `<h2>All queries</h2>
    <div class="meta">Every request that reached the warehouse. Click a row for its SQL.</div>
    <div class="toolbar">
      <input id="f-text" placeholder="Search titles, SQL, tables" oninput="drawRows()">
      <select id="f-mgr" onchange="drawRows()"><option value="">All teams</option>${mgrs.map(m => `<option>${esc(m)}</option>`).join('')}</select>
      <select id="f-verdict" onchange="drawRows()"><option value="">All verdicts</option>${verdicts.map(m => `<option>${esc(m)}</option>`).join('')}<option value="__warn">with warnings</option></select>
    </div>
    <div class="tablewrap"><table class="q"><thead><tr>
      ${[['manager','Team'],['night','Night'],['title','Report'],['engineer','Engineer'],['verdict','Verdict'],['tables','Tables'],['rows','Rows'],['elapsed_ms','Time'],['dry_runs','Dry runs']].map(([k, l]) => `<th onclick="sortBy('${k}')" class="${['rows','elapsed_ms','dry_runs','tables'].includes(k) ? 'num' : ''}">${l}</th>`).join('')}
    </tr></thead><tbody id="rows"></tbody></table></div>`;
}
function sortBy(k) { sortDir = sortKey === k ? -sortDir : 1; sortKey = k; drawRows(); }
function drawRows() {
  const text = ($('#f-text')?.value || '').toLowerCase(), mgr = $('#f-mgr')?.value, ver = $('#f-verdict')?.value;
  let qs = D.queries.map((q, i) => ({...q, i})).filter(q => (!mgr || q.manager === mgr) && (!ver || (ver === '__warn' ? (q.warnings || []).length > 0 : q.verdict === ver))
    && (!text || [q.title, q.sql, q.question, (q.tables || []).join(' ')].join(' ').toLowerCase().includes(text)));
  if (sortKey) qs.sort((a, b) => {
    const va = sortKey === 'tables' ? (a.tables || []).length : sortKey === 'night' ? a.week * 100 + a.night : a[sortKey];
    const vb = sortKey === 'tables' ? (b.tables || []).length : sortKey === 'night' ? b.week * 100 + b.night : b[sortKey];
    return (va > vb ? 1 : va < vb ? -1 : 0) * sortDir;
  });
  $('#rows').innerHTML = qs.map(q => `<tr class="main" onclick="toggleSql(${q.i})">
      <td>${esc(q.manager)}</td><td>w${q.week} n${q.night}</td><td>${esc(q.title)}</td><td>${esc(q.engineer)}</td>
      <td>${badge(q.verdict)}${(q.warnings || []).length ? ' ' + badge('warning') : ''}</td><td class="num" title="${esc((q.tables || []).join(', '))}">${(q.tables || []).length}</td>
      <td class="num">${q.rows ?? ''}</td><td class="num">${fmtMs(q.elapsed_ms)}</td><td class="num">${q.dry_runs ?? ''}</td></tr>
      <tr class="sqlrow" id="sql-${q.i}" hidden><td colspan="9"><div class="meta">${esc(q.question)}<br>Tables: ${esc((q.tables || []).join(', '))}${(q.reasons || []).length ? '<br>Reasons: ' + q.reasons.map(esc).join('; ') : ''}${(q.warnings || []).length ? '<br>Warnings (not shown to the agents): ' + q.warnings.map(esc).join('; ') : ''}</div>${sql(q.sql)}</td></tr>`).join('');
}
function toggleSql(i) { const r = $('#sql-' + i); r.hidden = !r.hidden; if (!r.hidden) highlight(r); }

function highlight(el = document) {
  if (window.hljs) el.querySelectorAll('pre code.language-sql:not(.hljs)').forEach(c => hljs.highlightElement(c));
}

let current = location.hash.slice(1) || 'overview';
document.addEventListener('click', e => {
  const a = e.target.closest('a[href^="#"]');
  if (!a) return;
  e.preventDefault();
  current = a.getAttribute('href').slice(1);
  try { history.pushState(null, '', '#' + current); } catch (err) {}
  route();
});
addEventListener('popstate', () => { current = location.hash.slice(1) || 'overview'; route(); });
function route() {
  const h = current;
  if (h.startsWith('track/')) { const id = h.slice(6); nav(id); $('#main').innerHTML = track(id); }
  else if (h === 'queries') { nav('queries'); $('#main').innerHTML = queries(); drawRows(); }
  else { nav('overview'); $('#main').innerHTML = overview(); }
  highlight();
  document.querySelectorAll('details').forEach(d => d.addEventListener('toggle', () => highlight(d)));
  window.scrollTo(0, 0);
}
route();
</script>
</body>
</html>
"""


def render(run):
    return PAGE.replace("__DATA__", json.dumps(build(Path(run)), ensure_ascii=False).replace("</", "<\\/"))


def main():
    if len(sys.argv) != 2:
        sys.exit(__doc__)
    run = Path(sys.argv[1])
    out = run / "index.html"
    out.write_text(render(run))
    print(f"wrote {out} ({out.stat().st_size // 1024} KB)")


if __name__ == "__main__":
    main()
