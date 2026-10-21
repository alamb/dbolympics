#!/usr/bin/env python3
"""Stand-in for `codex exec` used by `tools/roleplay.py --mock --backend codex`: canned answers.

It understands the flags the Codex backend passes and prints the JSON lines `codex exec --json`
prints (thread.started, item.started/completed, turn.completed). With an MCP server configured
(`-c mcp_servers.warehouse.*`) it plays a minimal engineer through the real warehouse tools;
with --output-schema it answers one canned report request. Tests the plumbing, not Codex.
"""

import json
import os
import subprocess
import sys
import time
import uuid

SQL = ("SELECT t.name AS merchant, count(*) AS orders\nFROM orders o\nJOIN tenants t ON t.id = o.tenant_id\n"
       "WHERE o.placed_at >= TIMESTAMP '2025-01-01 00:00:00'\nGROUP BY t.name\nORDER BY orders DESC, merchant")
DELAY = float(os.environ.get("ROLEPLAY_FAKE_DELAY") or 0)


def emit(ev):
    print(json.dumps(ev), flush=True)
    time.sleep(DELAY)


def main():
    argv = sys.argv[1:]
    assert argv and argv[0] == "exec", argv
    opts, conf, resume, i = {}, {}, None, 1
    while i < len(argv):
        a = argv[i]
        if a == "resume":
            resume = argv[i + 1]
            i += 2
        elif a == "-c":
            k, v = argv[i + 1].split("=", 1)
            conf[k] = json.loads(v)  # the backend writes TOML values that are also JSON
            i += 2
        elif a in ("-o", "-m", "-C", "--sandbox", "--output-schema", "--disable", "--enable"):
            opts[a] = argv[i + 1]
            i += 2
        else:
            opts[a] = True
            i += 1
    assert os.path.exists(conf["model_instructions_file"]), "the system prompt file must exist"
    sys.stdin.read()
    thread = resume or str(uuid.uuid4())
    emit({"type": "thread.started", "thread_id": thread})
    emit({"type": "turn.started"})
    if "mcp_servers.warehouse.command" in conf:
        p = subprocess.Popen([conf["mcp_servers.warehouse.command"]] + conf["mcp_servers.warehouse.args"],
                             stdin=subprocess.PIPE, stdout=subprocess.PIPE, text=True)

        def rpc(n, method, params):
            p.stdin.write(json.dumps({"jsonrpc": "2.0", "id": n, "method": method, "params": params}) + "\n")
            p.stdin.flush()
            return json.loads(p.stdout.readline())

        rpc(1, "initialize", {"protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": {"name": "fake-codex", "version": "0"}})
        for n, (tool, args) in enumerate([("dry_run", {"sql": SQL + " LIMT"}), ("dry_run", {"sql": SQL}), ("submit", {"sql": SQL, "note": ""})]):
            item = {"id": f"item_{n}", "type": "mcp_tool_call", "server": "warehouse", "tool": tool, "arguments": args, "status": "in_progress"}
            emit({"type": "item.started", "item": item})
            res = rpc(2 + n, "tools/call", {"name": tool, "arguments": args})["result"]
            emit({"type": "item.completed", "item": dict(item, status="completed", result={"content": res["content"]})})
        p.stdin.close()
        p.wait()
        text = "Submitted."
    elif "--output-schema" in opts and "headlines" in json.load(open(opts["--output-schema"]))["properties"]:
        text = json.dumps({"headlines": ["(mock) orders grew 12% in 2025", "(mock) refunds are flat at 3.5%"]})
    elif "--output-schema" in opts:
        text = json.dumps({"notes": "(mock)", "requests": [{
            "title": "Orders per merchant in 2025", "question": "Which merchants placed the most orders in 2025?",
            "grain": "merchant", "columns": ["merchant", "orders"], "details": "orders placed in 2025", "builds_on": []}]})
    else:
        text = "(mock text)"
    emit({"type": "item.completed", "item": {"id": "item_msg", "type": "agent_message", "text": text}})
    emit({"type": "turn.completed", "usage": {"input_tokens": 1000, "cached_input_tokens": 800, "output_tokens": 50}})
    with open(opts["-o"], "w") as f:
        f.write(text)


if __name__ == "__main__":
    main()
