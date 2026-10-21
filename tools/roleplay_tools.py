"""The engineers' warehouse tools of the role play: `dry_run` (free, no data) and `submit`.

Shared by the stdio MCP server that the CLI backends attach (tools/roleplay_mcp.py) and by the
API backend, which calls them directly (tools/roleplay_llm.py). Standard library only.
"""

import json
import subprocess

TOOLS = [
    {
        "name": "dry_run",
        "description": "Run a query against an empty copy of the warehouse schema: shows whether it works and lists its output columns. Free; never reads or returns data.",
        "schema": {"type": "object", "properties": {"sql": {"type": "string"}}, "required": ["sql"], "additionalProperties": False},
    },
    {
        "name": "submit",
        "description": "Submit the final query to tonight's batch. Call once; this ends your work on the request.",
        "schema": {
            "type": "object",
            "properties": {
                "sql": {"type": "string"},
                "note": {"type": "string", "description": "one or two sentences for the analyst if the query deviates from the request; empty otherwise"},
            },
            "required": ["sql", "note"],
            "additionalProperties": False,
        },
    },
]


class WarehouseTools:
    """The tools of one engineer request. `submit` writes the query to `state` (JSON)."""

    def __init__(self, bin, db, dry_log, title, question, tags, state):
        self.bin, self.db, self.dry_log = str(bin), str(db), str(dry_log)
        self.title, self.question, self.tags, self.state = title, question, tags, str(state)
        self.submitted = False
        self.dry_runs = 0

    def call(self, name, args):
        """Run one tool call; returns (text for the model, is_error)."""
        if name == "dry_run":
            return self.dry_run(args.get("sql", ""))
        if name == "submit":
            return self.submit(args.get("sql", ""), args.get("note", ""))
        return f"unknown tool {name}", True

    def dry_run(self, sql):
        self.dry_runs += 1
        cmd = [self.bin, "query", "--db", self.db, "--sql", "-", "--title", self.title, "--question", self.question, "--dry-run",
               "--log", self.dry_log, "--tags", json.dumps(dict(self.tags, step=self.dry_runs))]
        try:
            p = subprocess.run(cmd, input=sql, capture_output=True, text=True, timeout=120)
        except subprocess.TimeoutExpired:
            return "STATUS: REJECTED\n  - the dry run did not finish within 2 minutes on the empty schema (endless recursion?)", True
        if p.returncode != 0:
            return "STATUS: FAILED\n  - " + ((p.stderr.strip().splitlines() or ["unknown error"])[-1][:300]), True
        out = "\n".join(l for l in p.stdout.splitlines() if not l.startswith(("REPORT:", "QUESTION:")))
        return out, "STATUS: COMPILES" not in out

    def submit(self, sql, note):
        if self.submitted:
            return "Already submitted; your work on this request is done.", True
        self.submitted = True
        with open(self.state, "w") as f:
            json.dump({"sql": sql, "note": note, "dry_runs": self.dry_runs}, f)
        return "Submitted to tonight's batch. Your work on this request is done.", False
