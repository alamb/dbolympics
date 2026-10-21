#!/usr/bin/env python3
"""Stand-in for `claude -p` used by `tools/roleplay.py --mock`: canned answers, no model.

It understands the flags the orchestrator passes. With --mcp-config it plays a minimal
engineer: it starts the warehouse MCP server, dry-runs a fixed query and submits it. With
--json-schema it returns one canned report request. Otherwise it returns a line of text.
"""

import json
import os
import subprocess
import sys
import time
import uuid

# seconds to wait per step, to watch the progress output (ROLEPLAY_FAKE_DELAY=3)
DELAY = float(os.environ.get("ROLEPLAY_FAKE_DELAY") or 0)

SQL = ("SELECT t.name AS merchant, count(*) AS orders\nFROM orders o\nJOIN tenants t ON t.id = o.tenant_id\n"
       "WHERE o.placed_at >= TIMESTAMP '2025-01-01 00:00:00'\nGROUP BY t.name\nORDER BY orders DESC, merchant")
# another query to submit, e.g. a bundled one to test the warnings
SQL = os.environ.get("ROLEPLAY_FAKE_SQL") or SQL


def emit(ev):
    print(json.dumps(ev), flush=True)
    time.sleep(DELAY)


VALUED = {"--model", "--output-format", "--system-prompt-file", "--tools", "--mcp-config", "--json-schema", "--session-id",
          "--resume", "--setting-sources", "--permission-prompts", "--max-turns", "--allowedTools", "--effort"}


def parse(argv):
    opts, i = {}, 0
    while i < len(argv):
        a = argv[i]
        if a in VALUED:
            opts[a] = argv[i + 1]
            i += 2
        else:
            opts[a] = True
            i += 1
    return opts


def engineer(config_path):
    cfg = json.load(open(config_path))["mcpServers"]["warehouse"]
    p = subprocess.Popen([cfg["command"]] + cfg["args"], stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

    def rpc(i, method, params):
        p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": i, "method": method, "params": params}) + "\n")
        p.stdin.flush()
        return json.loads(p.stdout.readline())

    rpc(1, "initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "fake", "version": "0"}})
    assert {t["name"] for t in rpc(2, "tools/list", {})["result"]["tools"]} == {"dry_run", "submit"}
    for i, (name, args) in enumerate([("dry_run", {"sql": SQL + " LIMT"}), ("dry_run", {"sql": SQL}), ("submit", {"sql": SQL, "note": ""})]):
        emit({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": f"t{i}", "name": f"mcp__warehouse__{name}", "input": args}]}})
        res = rpc(3 + i, "tools/call", {"name": name, "arguments": args})["result"]
        emit({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": f"t{i}", "content": res["content"]}]}})
    p.stdin.close()
    p.wait()
    return "Submitted."


def main():
    o = parse(sys.argv[1:])
    sys.stdin.read()
    session = o.get("--session-id") or o.get("--resume") or str(uuid.uuid4())
    res = {"type": "result", "subtype": "success", "is_error": False, "session_id": session, "num_turns": 1,
           "total_cost_usd": 0, "usage": {"input_tokens": 0, "output_tokens": 0}}
    emit({"type": "system", "subtype": "init", "session_id": session})
    emit({"type": "assistant", "message": {"content": [{"type": "text", "text": "(thinking)"}]}})
    if "--mcp-config" in o:
        res["result"] = engineer(o["--mcp-config"])
    elif "--json-schema" in o and "headlines" in json.loads(o["--json-schema"])["properties"]:
        res["result"] = ""
        res["structured_output"] = {"headlines": ["(mock) orders grew 12% in 2025", "(mock) refunds are flat at 3.5%"]}
    elif "--json-schema" in o:
        res["result"] = ""
        res["structured_output"] = {"notes": "(mock)", "requests": [{
            "title": "Orders per merchant in 2025", "question": "Which merchants placed the most orders in 2025?",
            "grain": "merchant", "columns": ["merchant", "orders"], "details": "orders placed in 2025", "builds_on": []}]}
    else:
        res["result"] = "(mock text)"
    print(json.dumps(res))


if __name__ == "__main__":
    main()
