#!/usr/bin/env python3
"""The analytics-department role play that writes the benchmark queries (docs/QUERY-GENERATION.md).

For every manager persona one track runs, in parallel with the others:

  manager note -> analyst plans up to SLOTS report requests per night, for NIGHTS nights
  -> an engineer per request writes SQL (dry runs are free, one submit)
  -> the warehouse (`dbolympics query`) reviews, runs and prints the report, and logs lineage
  -> the analyst reads the reports, follows up, and writes a memo on Friday
  -> (optional further weeks) the manager replies to the memo with follow-up questions.

The agents run on one of three backends (tools/roleplay_llm.py), chosen with --backend:

- claude: headless Claude Code (`claude -p`), on the logged-in Claude subscription;
- codex: headless Codex (`codex exec`), on the logged-in ChatGPT/Codex account;
- api: any OpenAI-compatible chat completions API (--api-base, key from --api-key-env).

Whatever the backend, the agents see a system prompt built only from the files in roleplay/
(checked for revealing words first), their conversation, and for engineers the warehouse
tools `dry_run` and `submit` (tools/roleplay_tools.py; the CLIs get them as an MCP server).

    python3 tools/roleplay.py --db tmp/sf1.duckdb --out tmp/roleplay/run1 --managers ceo,ops
    python3 tools/roleplay.py --db tmp/sf1.duckdb --out tmp/roleplay/run2 --backend codex --model luna
    python3 tools/roleplay.py --db tmp/sf1.duckdb --out tmp/roleplay/run3 --backend api --api-base http://localhost:8000/v1 --model my-model
    python3 tools/roleplay.py --db tmp/sf1.duckdb --out tmp/roleplay/mock --mock [--backend codex|api]
"""

import argparse
import atexit
import concurrent.futures as cf
import json
import random
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import threading
import time
from pathlib import Path

from roleplay_llm import BACKENDS, Agent, Usage, elapsed
from roleplay_tools import WarehouseTools

ROOT = Path(__file__).resolve().parent.parent
RP = ROOT / "roleplay"
BIN = ROOT / "target" / "release" / "dbolympics"
MCP = ROOT / "tools" / "roleplay_mcp.py"

# words that would tell the agents they are not in a real company
REVEALING = re.compile(r"\b(bench\w*|planted|stor(y|ies)|simulat\w*|synthetic|fictional|fake|generated data|role.?play)\b", re.I)


# ---------------------------------------------------------------------------------------------
# documents


def read(p):
    return (RP / p).read_text()


def fill(template, **kw):
    return template.format(**kw)


def check_clean(text, where):
    m = REVEALING.search(text)
    if m:
        sys.exit(f"refusing to send {where}: it contains {m.group(0)!r}")


class Docs:
    def __init__(self):
        self.personas = json.loads(read("personas.json"))
        self.situations = json.loads(read("situations.json"))["situations"]
        self.roles = {p.stem: p.read_text() for p in (RP / "roles").glob("*.md")}
        # one shared prefix for every agent
        self.shared = f"{read('company.md')}\n\n{read('handbook.md')}\n\n# Table catalog\n\n{read('schema.md')}"
        for name, text in [("shared docs", self.shared), ("personas", json.dumps(self.personas)), ("situations", json.dumps(self.situations))] + list(self.roles.items()):
            check_clean(text, name)


def requests_schema(slots):
    return {
        "type": "object",
        "properties": {
            "notes": {"type": "string", "description": "your brief reasoning for tonight's requests"},
            "requests": {
                "type": "array",
                "maxItems": slots,
                "items": {
                    "type": "object",
                    "properties": {
                        "title": {"type": "string", "description": "report title"},
                        "question": {"type": "string", "description": "the one question this report answers"},
                        "grain": {"type": "string", "description": "what one row is, e.g. 'carrier and week'"},
                        "columns": {"type": "array", "items": {"type": "string"}, "description": "expected columns, each with its definition"},
                        "details": {"type": "string", "description": "filters, time window, definitions, tables you believe hold the answer, ordering"},
                        "builds_on": {"type": "array", "items": {"type": "string"}, "description": "titles of earlier reports this one follows up on (empty if none)"},
                    },
                    "required": ["title", "question", "grain", "columns", "details", "builds_on"],
                    "additionalProperties": False,
                },
            },
        },
        # strict form (every field required, no others): what OpenAI-style structured output demands
        "required": ["notes", "requests"],
        "additionalProperties": False,
    }


HEADLINES_SCHEMA = {
    "type": "object",
    "properties": {"headlines": {"type": "array", "maxItems": 3, "items": {"type": "string"}, "description": "one finding per line, with its key number"}},
    "required": ["headlines"],
    "additionalProperties": False,
}


# ---------------------------------------------------------------------------------------------
# progress


class Progress:
    """Prints events as they happen; when nothing was printed for a while, prints what is running."""

    def __init__(self, quiet_after=20):
        self.lock = threading.Lock()
        self.active = {}  # key -> [label, start, detail]
        self.quiet_after = quiet_after
        self.last = time.time()
        self.t0 = time.time()
        threading.Thread(target=self._tick, daemon=True).start()

    def log(self, msg):
        with self.lock:
            print(f"{time.strftime('%H:%M:%S')} {msg}", flush=True)
            self.last = time.time()

    def start(self, key, label, detail=""):
        with self.lock:
            self.active[key] = [label, time.time(), detail]

    def update(self, key, detail):
        with self.lock:
            if key in self.active:
                self.active[key][2] = detail

    def end(self, key):
        with self.lock:
            self.active.pop(key, None)

    def _tick(self):
        while True:
            time.sleep(2)
            with self.lock:
                now = time.time()
                if now - self.last < self.quiet_after or not self.active:
                    continue
                parts = [f"{label} ({detail + ', ' if detail else ''}{elapsed(now - start)})" for label, start, detail in self.active.values()]
                print(f"{time.strftime('%H:%M:%S')}   ... {elapsed(now - self.t0)} in, working on: " + "; ".join(parts), flush=True)
                self.last = now


# ---------------------------------------------------------------------------------------------
# agents


class Track:
    """One manager, one analyst, a pool of engineers, for one or more weeks."""

    def __init__(self, args, docs, llm, manager, analyst, out):
        self.a = args
        self.docs = docs
        self.llm = llm
        self.m = manager
        self.an = analyst
        self.out = out
        self.p = llm.progress
        self.agents = {}  # label -> Agent (conversation), for the transcripts afterwards
        self.memo = None
        # a business situation may land on the manager's desk: in week 1 instead of plain
        # onboarding, or later on top of the follow-up (at most one per track, by seed)
        own = [s for s in docs.situations if s["lead"] == manager["id"]]
        r = random.Random(f"{args.seed}|{manager['id']}|situation")
        self.situation, self.situation_week = None, None
        if own:
            self.situation_week = next((w for w in range(1, args.weeks + 1) if r.random() < args.situations), None)
            self.situation = r.choice(own) if self.situation_week else None

    def save(self, name, obj):
        p = self.out / name
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_text(obj if isinstance(obj, str) else json.dumps(obj, indent=1, ensure_ascii=False))
        return p

    def say(self, msg):
        self.p.log(f"[{self.m['id']}] {msg}")

    def busy(self, key, label):
        k = f"{self.m['id']}/{key}"
        self.p.start(k, f"{self.m['id']} {label}")
        return k

    def agent(self, label, prompt_name, role_text):
        """The conversation `label`, created on first use with the shared documents and a role."""
        if label not in self.agents:
            text = f"{self.docs.shared}\n\n# Your role\n\n{role_text}"
            check_clean(text, prompt_name)
            self.agents[label] = Agent(label, text, self.save(f"prompts/{prompt_name}.md", text))
        return self.agents[label]

    # -- engineer

    def engineer(self, eng, req, tags, key):
        agent = self.agent(f"engineer-{key}", f"engineer-{eng['name'].lower()}", fill(self.docs.roles["engineer"], **eng))
        cols = "\n".join(f"- {c}" for c in req.get("columns", []))
        msg = (f"Report request from {self.an['name']} (analyst for the {self.m['title']}):\n\n"
               f"Title: {req['title']}\nQuestion: {req['question']}\nGrain: {req['grain']}\n"
               f"Columns:\n{cols}\nDetails: {req.get('details', '')}\n\n"
               "Check your query with the warehouse's dry_run tool, then hand it in with submit.")
        state = self.out / "engineers" / f"{key}.submit.json"
        dry_log = self.a.out / "dry_runs.jsonl"
        # the tools: called in-process by the API backend, as an MCP server by the CLIs
        tools = WarehouseTools(BIN, self.a.db, dry_log, req["title"], req["question"], tags, state)
        mcp = self.save(f"engineers/{key}.mcp.json", {"mcpServers": {"warehouse": {"command": sys.executable, "args": [
            str(MCP), "--bin", str(BIN), "--db", str(self.a.db), "--dry-log", str(dry_log),
            "--title", req["title"], "--question", req["question"], "--tags", json.dumps(tags), "--state", str(state)]}}})
        who = f"{key} {eng['name']}"
        pkey = self.busy(key, f"{who} writing SQL")
        dry = [0]
        thread = []  # the engineer's working, whatever the backend (for the viewer)

        def on_event(kind, name=None, args=None, text=None):
            if kind == "text":
                thread.append({"kind": "text", "text": text})
            elif kind == "tool_call":
                thread.append({"kind": name, "sql": (args or {}).get("sql", ""), "note": (args or {}).get("note", "")})
                if name == "dry_run":
                    dry[0] += 1
                    self.p.update(pkey, f"dry run {dry[0]}")
                elif name == "submit":
                    self.say(f"{who}: submitted after {dry[0]} dry runs")
            elif kind == "tool_result":
                for t in reversed(thread):
                    if t["kind"] == name and "result" not in t:
                        t["result"] = text
                        break
                if name == "dry_run":
                    lines = [l.strip(" -") for l in (text or "").splitlines() if l.strip()]
                    status = lines[0].replace("STATUS: ", "").split(" (")[0] if lines else "?"
                    if status == "COMPILES":
                        cl = next((l for l in lines if l.startswith("COLUMNS:")), "")
                        detail = f"{len(cl.split(',')) if cl.strip() != 'COLUMNS:' else 0} columns"
                    else:
                        detail = lines[1][:110] if len(lines) > 1 else ""
                    self.say(f"{who}: dry run {dry[0]} -> {status.lower()}{': ' + detail if detail else ''}")

        try:
            self.llm.work(agent, msg, tools, mcp, self.a.engineer_turns, label=f"engineer {key}", key=pkey, on_event=on_event)
            if not state.exists():
                self.say(f"{who}: has not submitted, nudging")
                self.llm.work(agent, "You have not submitted yet. Submit your best query now with the submit tool.", tools, mcp, 4,
                              label=f"engineer {key} nudge", key=pkey, on_event=on_event)
        finally:
            self.p.end(pkey)
            self.save(f"engineers/{key}.thread.json", thread)
        if not state.exists():
            return "", "", 0
        s = json.loads(state.read_text())
        return s.get("sql", ""), s.get("note", ""), s.get("dry_runs", 0)

    def run_request(self, week, night, idx, req):
        # deterministic per request, independent of the order in which requests finish
        engineers = self.docs.personas["engineers"]
        eng = engineers[random.Random(f"{self.a.seed}|{self.m['id']}|{week}|{night}|{idx}").randrange(len(engineers))]
        tags = {
            "run": self.a.run_id, "manager": self.m["id"], "analyst": self.an["name"], "engineer": eng["name"],
            "situation": self.situation["id"] if self.situation and week >= self.situation_week else None,
            "week": week, "night": night, "request": idx, "grain": req.get("grain"),
            "columns_requested": req.get("columns"), "details": req.get("details"), "builds_on": req.get("builds_on", []),
        }
        sql, note, dry_runs = self.engineer(eng, req, tags, f"w{week}-n{night}-r{idx}")
        tags.update(engineer_note=note, dry_runs=dry_runs)
        if not sql.strip():
            report = f"REPORT: {req['title']}\nQUESTION: {req['question']}\nSTATUS: FAILED\n  - the engineer submitted no query\n"
        else:
            cmd = [str(BIN), "query", "--db", str(self.a.db), "--sql", "-", "--title", req["title"], "--question", req["question"],
                   "--log", str(self.a.out / "lineage.jsonl"), "--tags", json.dumps(tags)]
            try:
                p = subprocess.run(cmd, input=sql, capture_output=True, text=True, timeout=self.a.query_timeout)
                report = p.stdout if p.returncode == 0 else (
                    f"REPORT: {req['title']}\nQUESTION: {req['question']}\nSTATUS: FAILED\n  - "
                    + (p.stderr.strip().splitlines() or ["unknown error"])[-1][:300] + "\n")
            except subprocess.TimeoutExpired:
                p = subprocess.CompletedProcess(cmd, 1, "", "")
                report = (f"REPORT: {req['title']}\nQUESTION: {req['question']}\nSTATUS: FAILED\n"
                          f"  - cancelled: still running after {self.a.query_timeout} s\n")
        if "STATUS:" in report:
            status = report.split("STATUS:")[1].strip().splitlines()
            status = status[0].lower() + (": " + status[1].strip(" -")[:110] if len(status) > 1 else "")
        else:
            m = re.search(r"\((\d+) rows", report)
            status = f"report printed ({m.group(1) if m else '?'} rows)"
        self.say(f"w{week}-n{night}-r{idx} {eng['name']}: {status}: {req['title']}")
        if sql.strip():
            # signs of bundling the warehouse no longer bounces: for us only, never sent back
            for w in (l[len("WARNING: "):] for l in p.stderr.splitlines() if l.startswith("WARNING: ")):
                self.say(f"w{week}-n{night}-r{idx} WARNING (not shown to agents): {w}")
        head = f"=== Report {idx}: {req['title']} (written by {eng['name']}) ===\n"
        if note:
            head += f"Engineer's note: {note}\n"
        return head + report

    # -- analyst

    def analyst_week(self, week, opening):
        role = fill(self.docs.roles["analyst"], name=self.an["name"], style=self.an["style"], manager_name=self.m["name"],
                    manager_title=self.m["title"], slots=self.a.slots, rounds=self.a.nights)
        agent = self.agent("analyst", "analyst", role)
        schema = requests_schema(self.a.slots)
        prompt = opening + f"\n\nNight 1 of {self.a.nights}: you have {self.a.slots} report slots tonight."
        for night in range(1, self.a.nights + 1):
            self.say(f"analyst {self.an['name']} is planning week {week}, night {night}")
            pkey = self.busy("analyst", f"analyst planning w{week}-n{night}")
            try:
                reply = self.llm.ask(agent, prompt, schema=schema, label=f"analyst w{week} n{night}", key=pkey)
            finally:
                self.p.end(pkey)
            plan = reply.data or {}
            reqs = [r for r in (plan.get("requests") or []) if isinstance(r, dict) and r.get("title") and r.get("question")][: self.a.slots]
            self.save(f"week{week}-night{night}-plan.json", plan)
            self.say(f"night {night} plan: {len(reqs)} request{'' if len(reqs) == 1 else 's'}" + "".join(f"\n           r{i + 1}: {r['title']}" for i, r in enumerate(reqs)))
            with cf.ThreadPoolExecutor(max_workers=max(1, len(reqs))) as ex:
                reports = list(ex.map(lambda ir: self.run_request(week, night, ir[0] + 1, ir[1]), enumerate(reqs)))
            body = "\n".join(reports) if reports else "No reports were requested tonight."
            self.save(f"week{week}-night{night}-reports.txt", body)
            if night < self.a.nights:
                nxt = f"Night {night + 1} of {self.a.nights}: you have {self.a.slots} report slots tonight."
            else:
                nxt = fill(self.docs.roles["analyst-memo"], manager_name=self.m["name"])
            prompt = f"The printed reports of night {night}:\n\n{body}\n\n{nxt}"
        self.say(f"analyst {self.an['name']} is writing the week {week} memo")
        pkey = self.busy("analyst", f"analyst writing memo w{week}")
        try:
            reply = self.llm.ask(agent, prompt, label=f"analyst w{week} memo", key=pkey)
        finally:
            self.p.end(pkey)
        return reply.text

    # -- manager

    def manager_note(self, memo, week, team):
        agent = self.agent("manager", f"manager-{self.m['id']}", f"You are {self.m['name']}, {self.m['title']} at Tidewick Commerce.")
        arrives = self.situation is not None and self.situation_week == week
        with_text = ""
        if arrives:
            managers = {m["id"]: m for m in self.docs.personas["managers"]}
            peers = [f"{managers[i]['name']} ({managers[i]['title']})" for i in self.situation["with"] if i in managers]
            with_text = f" You lead this together with {' and '.join(peers)}." if peers else ""
        if memo is None:
            if arrives:
                prompt = fill(self.docs.roles["manager-situation"], analyst=self.an["name"], situation=self.situation["text"], with_text=with_text, **self.m)
            else:
                prompt = fill(self.docs.roles["manager"], analyst=self.an["name"], **self.m)
        else:
            news = (f"\nAlso, something came up this week: {self.situation['text']}{with_text}\n\n"
                    "Cover it in your note as well: what it requires, and which decision or deliverable it feeds.\n") if arrives else ""
            team_text = ("\nHeadlines your direct reports' teams posted this week:\n" + "\n".join(team) + "\n") if team else ""
            prompt = fill(self.docs.roles["manager-followup"], analyst=self.an["name"], memo=memo, news=news, team=team_text)
        self.say(f"{self.m['title']} {self.m['name']} is writing to the analyst")
        pkey = self.busy("manager", "manager writing")
        try:
            reply = self.llm.ask(agent, prompt, label="manager", key=pkey)
        finally:
            self.p.end(pkey)
        return reply.text

    # -- co-owners and the analytics channel

    def coowner_notes(self, week):
        """Notes from the co-owners of a situation that arrives this week, each to this analyst."""
        if not (self.situation and self.situation_week == week and self.situation["with"]):
            return []
        managers = {m["id"]: m for m in self.docs.personas["managers"]}
        notes = []
        for cid in self.situation["with"]:
            c = managers.get(cid)
            if not c:
                continue
            agent = self.agent(f"coowner-{cid}", f"coowner-{cid}", f"You are {c['name']}, {c['title']} at Tidewick Commerce.")
            prompt = fill(self.docs.roles["coowner"], lead_name=self.m["name"], lead_title=self.m["title"], situation=self.situation["text"],
                          analyst=self.an["name"], **c)
            self.say(f"co-owner {c['title']} {c['name']} is writing to the analyst")
            pkey = self.busy(f"coowner-{cid}", f"co-owner {cid} writing")
            try:
                text = self.llm.ask(agent, prompt, label=f"co-owner {cid}", key=pkey).text
            finally:
                self.p.end(pkey)
            self.save(f"week{week}-coowner-{cid}.md", text)
            notes.append(f"Note from {c['name']} ({c['title']}), who co-owns this with {self.m['name']}:\n\n{text}")
        return notes

    def headlines(self, week):
        """The analyst's post to the company's analytics channel at the end of the week."""
        pkey = self.busy("analyst", f"analyst posting headlines w{week}")
        try:
            reply = self.llm.ask(self.agents["analyst"], self.docs.roles["analyst-headlines"], schema=HEADLINES_SCHEMA, label=f"analyst w{week} headlines", key=pkey)
        finally:
            self.p.end(pkey)
        heads = [h.strip() for h in (reply.data or {}).get("headlines", []) if isinstance(h, str) and h.strip()][:3]
        self.save(f"week{week}-headlines.json", heads)
        return heads

    # -- the whole track, week by week (all tracks move in step, see main)

    def start(self):
        if self.situation:
            self.save("situation.json", dict(self.situation, week=self.situation_week))
            self.say(f"situation from week {self.situation_week}: {self.situation['id']}" + (f" (with {', '.join(self.situation['with'])})" if self.situation["with"] else ""))
        else:
            self.say("no situation: onboarding only")

    def week(self, week, channel, team):
        """One week: the manager's note (with the team's headlines), co-owner notes, four nights
        of reports (the channel's last digest in view), the memo and this week's headlines."""
        if week == 1:
            self.start()
        note = self.manager_note(self.memo, week, team)
        self.save(f"week{week}-manager-note.md", note)
        self.say(f"week {week}: manager note written ({len(note)} chars)")
        if week == 1:
            opening = f"Monday 2026-01-05. Note from {self.m['name']} ({self.m['title']}):\n\n{note}"
        else:
            opening = f"Monday, week {week}. {self.m['name']} replied to your memo:\n\n{note}"
        for n in self.coowner_notes(week):
            opening += "\n\n" + n
        if channel:
            opening += f"\n\nLast week in the company's analytics channel:\n{channel}"
        self.memo = self.analyst_week(week, opening)
        self.save(f"week{week}-memo.md", self.memo)
        self.say(f"week {week}: memo written ({len(self.memo)} chars) -> {self.out / f'week{week}-memo.md'}")
        heads = self.headlines(week)
        self.say(f"week {week}: posted {len(heads)} headlines")
        return heads

    def finish(self):
        self.save("sessions.json", {label: a.session for label, a in self.agents.items() if a.session})
        self.llm.save_transcripts(self.agents, self.out / "transcripts")


# ---------------------------------------------------------------------------------------------


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--db", required=True, type=Path, help="DuckDB file built by `dbolympics load`")
    ap.add_argument("--out", required=True, type=Path, help="output directory (must not exist)")
    ap.add_argument("--backend", choices=sorted(BACKENDS), default="claude", help="who plays the agents (see above)")
    ap.add_argument("--model", default=None, help="model name (claude default: sonnet; codex default: Codex's own; api: required)")
    ap.add_argument("--effort", default="", help="optional reasoning effort (claude --effort / codex model_reasoning_effort)")
    ap.add_argument("--claude", default="claude", help="the Claude Code executable")
    ap.add_argument("--codex", default="codex", help="the Codex executable")
    ap.add_argument("--codex-user-config", action="store_true", help="load your Codex config.toml (automatic when /etc/codex/requirements.toml demands its permission profiles)")
    ap.add_argument("--api-base", default=None, help="OpenAI-compatible base URL (default: $OPENAI_BASE_URL or https://api.openai.com/v1)")
    ap.add_argument("--api-key-env", default="OPENAI_API_KEY", help="environment variable holding the API key")
    ap.add_argument("--managers", default="", help="comma-separated manager ids (default: all)")
    ap.add_argument("--nights", type=int, default=4)
    ap.add_argument("--slots", type=int, default=3)
    ap.add_argument("--weeks", type=int, default=1)
    ap.add_argument("--engineer-turns", type=int, default=25, help="max agent turns per engineer request")
    ap.add_argument("--query-timeout", type=int, default=600, help="seconds before a report query is cancelled")
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--situations", type=float, default=0.5, help="chance per week that a business situation (roleplay/situations.json) lands on a track's manager; 0 = onboarding only")
    ap.add_argument("--parallel", type=int, default=3, help="tracks running at once")
    ap.add_argument("--mock", action="store_true", help="canned answers instead of a model (a stand-in CLI or API, per backend)")
    args = ap.parse_args()

    if not BIN.exists():
        sys.exit(f"{BIN} not found: run `cargo build --release` first")
    if not args.db.exists() or not args.db.with_suffix(".empty.duckdb").exists():
        sys.exit(f"{args.db} or its empty copy not found: run `dbolympics load <dir> {args.db}`")
    if args.out.exists():
        sys.exit(f"{args.out} exists; choose a new output directory")
    exe = {"claude": args.claude, "codex": args.codex}.get(args.backend)
    if exe and not args.mock and not shutil.which(exe):
        sys.exit(f"{exe} not found on PATH")
    args.db = args.db.resolve()
    args.out = args.out.resolve()
    args.out.mkdir(parents=True)
    args.run_id = args.out.name

    docs = Docs()
    usage = Usage()
    progress = Progress()
    # an empty directory outside the repository: nothing to discover, no project settings
    workdir = tempfile.mkdtemp(prefix="analytics-")
    if args.mock and args.backend == "api":
        # a stand-in chat completions server on a free local port
        with socket.socket() as sk:
            sk.bind(("127.0.0.1", 0))
            port = sk.getsockname()[1]
        server = subprocess.Popen([sys.executable, str(ROOT / "tools" / "roleplay_fake_openai.py"), str(port)],
                                  stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        atexit.register(server.terminate)
        args.api_base, args.model = f"http://127.0.0.1:{port}/v1", args.model or "mock"
        time.sleep(0.5)
    llm = BACKENDS[args.backend](args, usage, progress, workdir)

    managers = docs.personas["managers"]
    if args.managers:
        wanted = args.managers.split(",")
        unknown = set(wanted) - {m["id"] for m in managers}
        if unknown:
            sys.exit(f"unknown managers: {', '.join(sorted(unknown))}")
        managers = [m for m in managers if m["id"] in wanted]
    analysts = docs.personas["analysts"]
    tracks = [Track(args, docs, llm, m, analysts[i % len(analysts)], args.out / m["id"]) for i, m in enumerate(managers)]

    (args.out / "run.json").write_text(json.dumps({k: str(v) for k, v in vars(args).items()}, indent=1))
    (args.out / "personas.json").write_text(json.dumps(docs.personas, indent=1, ensure_ascii=False))
    progress.log(f"{len(tracks)} tracks ({', '.join(t.m['id'] for t in tracks)}), {args.parallel} at a time; "
                 f"{args.weeks} week(s) x {args.nights} nights x {args.slots} slots; backend {args.backend}, model {args.model or 'default'}; output in {args.out}")
    t0 = time.time()
    failed = []
    # weeks run in step: the channel digest and the team headlines of a week exist only once
    # every track has finished it
    alive = list(tracks)
    heads = {}  # manager id -> last week's headlines
    managers = {m["id"]: m for m in docs.personas["managers"]}
    channel = ""
    try:
        for week in range(1, args.weeks + 1):
            def team_of(t):
                return [f"- {managers[mid]['title']} ({managers[mid]['name']}): {h}"
                        for mid, hs in heads.items() if managers[mid].get("reports_to") == t.m["id"] for h in hs]
            with cf.ThreadPoolExecutor(max_workers=args.parallel) as ex:
                futs = {ex.submit(t.week, week, channel, team_of(t)): t for t in alive}
                new = {}
                for f in cf.as_completed(futs):
                    t = futs[f]
                    try:
                        new[t.m["id"]] = f.result()
                        progress.log(f"[{t.m['id']}] week {week} done")
                    except Exception as e:  # keep the other tracks going
                        failed.append(t.m["id"])
                        alive.remove(t)
                        progress.log(f"[{t.m['id']}] TRACK FAILED: {e}")
            heads = new
            lines = [f"- {managers[mid]['title']} team ({t.an['name']}): {h}" for t in alive for mid in [t.m["id"]] for h in heads.get(mid, [])]
            channel = "\n".join(lines)
            (args.out / f"week{week}-channel.md").write_text(f"# Analytics channel, week {week}\n\n{channel}\n")
            progress.log(f"week {week} done for all tracks: {len(lines)} headlines in the analytics channel")
    finally:
        for t in tracks:
            t.finish()

    summary = dict(usage.summary(), seconds=round(time.time() - t0, 1), failed_tracks=failed)
    verdicts = {}
    lineage = args.out / "lineage.jsonl"
    if lineage.exists():
        for line in lineage.read_text().splitlines():
            v = json.loads(line)["verdict"]
            verdicts[v] = verdicts.get(v, 0) + 1
    summary["reports"] = verdicts
    (args.out / "usage.json").write_text(json.dumps(summary, indent=1))
    progress.log(f"finished in {elapsed(summary['seconds'])}: reports {verdicts or 'none'}; {usage.calls} {args.backend} calls, "
                 f"{usage.turns} turns, API-equivalent ${summary['api_equivalent_usd']}" + (f"; FAILED tracks: {failed}" if failed else ""))
    try:
        import roleplay_viewer  # tools/ is on the path when this script runs
        page = args.out / "index.html"
        page.write_text(roleplay_viewer.render(args.out))
        progress.log(f"browse the run: open {page}")
    except Exception as e:  # the run itself is complete either way
        progress.log(f"could not build the viewer page: {e}")
    return 1 if failed else 0


if __name__ == "__main__":
    sys.exit(main())
