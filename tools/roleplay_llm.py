"""Model backends of the role play (tools/roleplay.py): Claude Code, Codex, or an OpenAI-compatible API.

Every backend offers the same two calls on an `Agent` (a conversation with a fixed system prompt):

- `ask(agent, prompt, schema=None)`: one turn; with a JSON schema the reply is structured data.
- `work(agent, prompt, tools, mcp_config)`: a turn in which the agent may use the warehouse
  tools (`dry_run`, `submit`) until it stops; tool calls are reported through `on_event` as
  ("tool_call", name, args) and ("tool_result", name, text), the agent's text as ("text", text).

The CLI backends attach the tools as an MCP server and keep conversations as CLI sessions
(resumed by id); the API backend calls the tools itself and keeps the message list. Isolation
is the same everywhere: the agents see their system prompt (built from roleplay/), the
conversation and the warehouse tools, nothing else. Standard library only.
"""

import json
import os
import random
import re
import shutil
import subprocess
import sys
import threading
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

from roleplay_tools import TOOLS


class Agent:
    """One conversation: a system prompt (text and the file it is saved in) and its history."""

    def __init__(self, label, system, system_file):
        self.label = label
        self.system = system
        self.system_file = Path(system_file)
        self.session = None  # CLI session id
        self.messages = []  # API conversation


class Reply:
    def __init__(self, text, data=None):
        self.text = text
        self.data = data


def parse_json(text):
    """The last JSON object in a text (structured replies without native support)."""
    for m in reversed(list(re.finditer(r"\{.*\}", text or "", re.S))):
        try:
            return json.loads(m.group(0))
        except json.JSONDecodeError:
            continue
    return None


def elapsed(seconds):
    m, s = divmod(int(seconds), 60)
    return f"{m}m{s:02d}s" if m else f"{s}s"


class Usage:
    KEYS = ("input_tokens", "cache_creation_input_tokens", "cache_read_input_tokens", "output_tokens")

    def __init__(self):
        self.lock = threading.Lock()
        self.calls = 0
        self.turns = 0
        self.cost_usd = 0.0  # what Claude Code reports the calls would cost at API prices
        self.tokens = dict.fromkeys(self.KEYS, 0)

    def add(self, turns=0, cost=0.0, **tokens):
        with self.lock:
            self.calls += 1
            self.turns += turns
            self.cost_usd += cost or 0.0
            for k in self.KEYS:
                self.tokens[k] += tokens.get(k) or 0

    def summary(self):
        return {"calls": self.calls, "turns": self.turns, "tokens": self.tokens, "api_equivalent_usd": round(self.cost_usd, 2)}


class Backend:
    name = "?"

    def __init__(self, args, usage, progress, workdir):
        self.a = args
        self.usage = usage
        self.progress = progress
        self.workdir = workdir

    def retry_wait(self, attempt, label, err):
        wait = min(900, 30 * 2 ** attempt) + random.random() * 5
        last = err.strip().splitlines()[-1][:200] if err.strip() else ""
        self.progress.log(f"[{label}] {self.name} failed: {last}; retrying in {elapsed(wait)}")
        time.sleep(wait)

    def save_transcripts(self, agents, dest):
        """Copy or write the conversations of a track next to its outputs."""


# ---------------------------------------------------------------------------------------------
# Claude Code (`claude -p`)


class ClaudeCLI(Backend):
    name = "claude"
    TOOL_NAMES = "mcp__warehouse__dry_run,mcp__warehouse__submit"

    def __init__(self, args, usage, progress, workdir):
        super().__init__(args, usage, progress, workdir)
        self.prefix = [sys.executable, str(Path(__file__).with_name("roleplay_fake_claude.py"))] if args.mock else [args.claude]

    def ask(self, agent, prompt, schema=None, label="", key=None):
        res = self._run(agent, prompt, schema=schema, max_turns=4, label=label, key=key)
        data = None
        if schema:
            data = res["structured_output"] if isinstance(res.get("structured_output"), dict) else parse_json(res.get("result"))
        return Reply((res.get("result") or "").strip(), data)

    def work(self, agent, prompt, tools, mcp_config, max_turns, label="", key=None, on_event=None):
        calls = {}  # tool_use id -> tool name

        def events(ev):
            for b in (ev.get("message") or {}).get("content") or []:
                if not isinstance(b, dict) or not on_event:
                    continue
                if ev.get("type") == "assistant" and b.get("type") == "text" and b.get("text", "").strip():
                    on_event("text", text=b["text"].strip())
                elif b.get("type") == "tool_use":
                    name = b.get("name", "").rsplit("__", 1)[-1]
                    calls[b.get("id")] = name
                    on_event("tool_call", name=name, args=b.get("input") or {})
                elif b.get("type") == "tool_result" and b.get("tool_use_id") in calls:
                    c = b.get("content")
                    text = c if isinstance(c, str) else " ".join(x.get("text", "") for x in c or [] if isinstance(x, dict))
                    on_event("tool_result", name=calls[b["tool_use_id"]], text=text)

        res = self._run(agent, prompt, mcp=mcp_config, max_turns=max_turns, label=label, key=key, on_event=events)
        return Reply((res.get("result") or "").strip())

    def _run(self, agent, prompt, schema=None, mcp=None, max_turns=4, label="", key=None, on_event=None):
        err = ""
        for attempt in range(7):
            resume = agent.session is not None
            session = agent.session or str(uuid.uuid4())
            cmd = self.prefix + ["-p", "--model", self.a.model or "sonnet", "--output-format", "stream-json", "--verbose",
                                 "--system-prompt-file", str(agent.system_file),
                                 "--setting-sources", "", "--strict-mcp-config", "--disable-slash-commands",
                                 "--permission-prompts", "none", "--max-turns", str(max_turns)]
            cmd += ["--resume", session] if resume else ["--session-id", session]
            if self.a.effort:
                cmd += ["--effort", self.a.effort]
            if schema:
                cmd += ["--json-schema", json.dumps(schema)]
            if mcp:
                cmd += ["--mcp-config", str(mcp), "--allowedTools", self.TOOL_NAMES]
            cmd += ["--tools", ""]  # last: it takes a list of names
            # fewer of Claude Code's own extras in the context (task nudges, skill list)
            env = dict(os.environ, CLAUDE_CODE_DISABLE_ATTACHMENTS="1")
            res, err, turns = stream(cmd, prompt, self.workdir, env, self.progress, key, on_event,
                                     is_result=lambda ev: ev.get("type") == "result",
                                     is_turn=lambda ev: ev.get("type") == "assistant")
            if res is not None and (not res.get("is_error") or res.get("subtype") == "error_max_turns"):
                agent.session = res.get("session_id", session)
                u = res.get("usage") or {}
                self.usage.add(turns=res.get("num_turns") or 0, cost=res.get("total_cost_usd"), **{k: u.get(k) for k in Usage.KEYS})
                return res
            err = ((res or {}).get("result") or err or "no output").strip()[-500:]
            if "not logged in" in err.lower():
                break
            if not resume:
                agent.session = None  # a failed first turn may have claimed the id: start afresh
            self.retry_wait(attempt, label, err)
        raise RuntimeError(f"claude failed ({label}): {err}")

    def save_transcripts(self, agents, dest):
        base = Path(os.environ.get("CLAUDE_CONFIG_DIR") or Path.home() / ".claude") / "projects"
        for label, agent in agents.items():
            for f in base.glob(f"*/{agent.session}.jsonl") if agent.session else []:
                dest.mkdir(parents=True, exist_ok=True)
                shutil.copy(f, dest / f"{label}.jsonl")


def stream(cmd, prompt, cwd, env, progress, key, on_event, is_result, is_turn):
    """Run a CLI that prints JSON lines; returns (result event, stderr and stray output, turns)."""
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, cwd=cwd, env=env)
    # stderr is read on the side so a chatty CLI cannot block on a full pipe
    stderr = []
    t = threading.Thread(target=lambda: stderr.append(p.stderr.read()), daemon=True)
    t.start()
    p.stdin.write(prompt)
    p.stdin.close()
    res, other, turns = None, [], 0
    for line in p.stdout:
        try:
            ev = json.loads(line)
        except json.JSONDecodeError:
            other.append(line)
            continue
        if is_result(ev):
            res = ev
        if is_turn(ev):
            turns += 1
            if key:
                progress.update(key, f"turn {turns}")
        if on_event:
            on_event(ev)
    p.wait()
    t.join(timeout=5)
    return res, ("".join(stderr) + "".join(other)), turns


# ---------------------------------------------------------------------------------------------
# Codex (`codex exec`)


def toml(value):
    """A value for `codex -c key=value` (TOML; JSON strings are valid TOML basic strings)."""
    if isinstance(value, list):
        return "[" + ", ".join(toml(v) for v in value) + "]"
    return json.dumps(value)


class CodexCLI(Backend):
    """`codex exec --json` with Codex's own instructions replaced by the agent's system prompt
    (`model_instructions_file`), no shell or web tools, a read-only sandbox, no user config or
    rules, and the warehouse tools as an MCP server."""

    name = "codex"

    # managed (admin/MDM) requirements can demand permission profiles that only the user's
    # config.toml defines; then Codex refuses to start with --ignore-user-config
    REQUIREMENTS = Path("/etc/codex/requirements.toml")

    def __init__(self, args, usage, progress, workdir):
        super().__init__(args, usage, progress, workdir)
        self.prefix = [sys.executable, str(Path(__file__).with_name("roleplay_fake_codex.py"))] if args.mock else [args.codex]
        req = self.REQUIREMENTS.read_text() if self.REQUIREMENTS.exists() and not args.mock else ""
        self.user_config = getattr(args, "codex_user_config", False) or "allowed_permission_profiles" in req or "default_permissions" in req
        if self.user_config:
            progress.log(f"codex: loading your Codex user config ({self.REQUIREMENTS} requires its permission profiles); "
                         "its MCP servers, developer instructions, notifications and context extras are switched off")

    def ask(self, agent, prompt, schema=None, label="", key=None):
        text = self._run(agent, prompt, schema=schema, label=label, key=key)
        return Reply(text, parse_json(text) if schema else None)

    def work(self, agent, prompt, tools, mcp_config, max_turns, label="", key=None, on_event=None):
        server = json.loads(Path(mcp_config).read_text())["mcpServers"]["warehouse"]

        def events(ev):
            item = ev.get("item") or {}
            if not on_event:
                return
            if ev.get("type") == "item.completed" and item.get("type") == "agent_message" and (item.get("text") or "").strip():
                on_event("text", text=item["text"].strip())
            elif item.get("type") == "mcp_tool_call":
                name = item.get("tool") or ""
                if ev.get("type") == "item.started":
                    args = item.get("arguments")
                    on_event("tool_call", name=name, args=json.loads(args) if isinstance(args, str) else (args or {}))
                elif ev.get("type") == "item.completed":
                    result = item.get("result") or {}
                    content = result.get("content") if isinstance(result, dict) else None
                    text = " ".join(c.get("text", "") for c in content or [] if isinstance(c, dict)) or str((item.get("error") or {}).get("message", ""))
                    on_event("tool_result", name=name, text=text)

        text = self._run(agent, prompt, mcp=server, label=label, key=key, on_event=events)
        return Reply(text)

    def _run(self, agent, prompt, schema=None, mcp=None, label="", key=None, on_event=None):
        err = ""
        for attempt in range(7):
            last = Path(self.workdir) / f"last-{uuid.uuid4().hex}.txt"
            cmd = self.prefix + ["exec", "--json", "-o", str(last), "--skip-git-repo-check", "--sandbox", "read-only",
                                 "--ignore-rules", "-C", str(self.workdir),
                                 "-c", f"model_instructions_file={toml(str(agent.system_file))}",
                                 "--disable", "shell_tool", "--disable", "unified_exec",
                                 "-c", 'web_search="disabled"', "-c", 'approval_policy="never"',
                                 # no extra context beyond the system prompt and the conversation
                                 "-c", "include_environment_context=false", "-c", "include_permissions_instructions=false",
                                 "-c", "include_apps_instructions=false", "-c", "include_collaboration_mode_instructions=false",
                                 "-c", "project_doc_max_bytes=0"]
            if self.user_config:
                # the user's config is needed (see __init__): neutralise what it could add
                cmd += ["-c", "mcp_servers={}", "-c", 'developer_instructions=""', "-c", "notify=[]"]
            else:
                cmd += ["--ignore-user-config"]
            if self.a.model:
                cmd += ["-m", self.a.model]
            if self.a.effort:
                cmd += ["-c", f"model_reasoning_effort={toml(self.a.effort)}"]
            if schema:
                sf = Path(self.workdir) / f"schema-{uuid.uuid4().hex}.json"
                sf.write_text(json.dumps(schema))
                cmd += ["--output-schema", str(sf)]
            if mcp:
                cmd += ["-c", f"mcp_servers.warehouse.command={toml(mcp['command'])}",
                        "-c", f"mcp_servers.warehouse.args={toml(mcp['args'])}",
                        "-c", 'mcp_servers.warehouse.default_tools_approval_mode="approve"']
            cmd += ["resume", agent.session, "-"] if agent.session else ["-"]
            state = {"thread": None, "usage": {}, "failed": None, "messages": []}

            def track(ev):
                if ev.get("type") == "thread.started":
                    state["thread"] = ev.get("thread_id")
                elif ev.get("type") == "turn.completed":
                    for k, v in (ev.get("usage") or {}).items():
                        state["usage"][k] = state["usage"].get(k, 0) + (v or 0)
                elif ev.get("type") in ("turn.failed", "error"):
                    state["failed"] = json.dumps(ev.get("error") or ev.get("message") or ev)
                item = ev.get("item") or {}
                if ev.get("type") == "item.completed" and item.get("type") == "agent_message":
                    state["messages"].append(item.get("text") or "")
                if on_event:
                    on_event(ev)

            _, stray, turns = stream(cmd, prompt, self.workdir, dict(os.environ), self.progress, key, track,
                                     is_result=lambda ev: False,
                                     is_turn=lambda ev: ev.get("type") == "item.completed" and (ev.get("item") or {}).get("type") == "agent_message")
            text = last.read_text().strip() if last.exists() else (state["messages"][-1].strip() if state["messages"] else "")
            last.unlink(missing_ok=True)
            if state["failed"] is None and (text or state["messages"]):
                agent.session = agent.session or state["thread"]
                u = state["usage"]
                self.usage.add(turns=turns, input_tokens=(u.get("input_tokens") or 0) - (u.get("cached_input_tokens") or 0),
                               cache_read_input_tokens=u.get("cached_input_tokens"), output_tokens=u.get("output_tokens"))
                return text
            err = (state["failed"] or stray or "no output").strip()[-500:]
            # configuration, login and model errors do not go away by waiting
            if any(k in err.lower() for k in ("codex login", "requirements.toml", "config.toml", "unknown model", "model_not_found", "not supported")):
                break
            self.retry_wait(attempt, label, err)
        raise RuntimeError(f"codex failed ({label}): {err}")

    def save_transcripts(self, agents, dest):
        base = Path(os.environ.get("CODEX_HOME") or Path.home() / ".codex") / "sessions"
        for label, agent in agents.items():
            for f in base.rglob(f"*{agent.session}*.jsonl") if agent.session and base.exists() else []:
                dest.mkdir(parents=True, exist_ok=True)
                shutil.copy(f, dest / f"{label}.jsonl")


# ---------------------------------------------------------------------------------------------
# OpenAI-compatible chat completions API


class OpenAIAPI(Backend):
    """POST {base}/chat/completions with function tools; structured replies via a JSON-schema
    response format (falling back to "reply with JSON" where a server does not support it)."""

    name = "api"

    def __init__(self, args, usage, progress, workdir):
        super().__init__(args, usage, progress, workdir)
        self.base = (args.api_base or os.environ.get("OPENAI_BASE_URL") or "https://api.openai.com/v1").rstrip("/")
        self.key = os.environ.get(args.api_key_env, "")
        self.schema_ok = True
        if not args.model:
            sys.exit("--model is required with --backend api")

    def ask(self, agent, prompt, schema=None, label="", key=None):
        self._start(agent)
        agent.messages.append({"role": "user", "content": prompt})
        msg = self._chat(agent.messages, schema=schema, label=label)
        agent.messages.append({"role": "assistant", "content": msg.get("content") or ""})
        text = (msg.get("content") or "").strip()
        return Reply(text, parse_json(text) if schema else None)

    def work(self, agent, prompt, tools, mcp_config, max_turns, label="", key=None, on_event=None):
        self._start(agent)
        agent.messages.append({"role": "user", "content": prompt})
        specs = [{"type": "function", "function": {"name": t["name"], "description": t["description"], "parameters": t["schema"]}} for t in TOOLS]
        text = ""
        for turn in range(max_turns):
            if key:
                self.progress.update(key, f"turn {turn + 1}")
            msg = self._chat(agent.messages, tools=specs, label=label)
            calls = msg.get("tool_calls") or []
            agent.messages.append({k: v for k, v in {"role": "assistant", "content": msg.get("content") or "", "tool_calls": calls or None}.items() if v is not None})
            if (msg.get("content") or "").strip():
                text = msg["content"].strip()
                if on_event:
                    on_event("text", text=text)
            if not calls:
                break
            for c in calls:
                name = c["function"]["name"]
                try:
                    args = json.loads(c["function"].get("arguments") or "{}")
                except json.JSONDecodeError:
                    args = {}
                if on_event:
                    on_event("tool_call", name=name, args=args)
                result, _ = tools.call(name, args)
                if on_event:
                    on_event("tool_result", name=name, text=result)
                agent.messages.append({"role": "tool", "tool_call_id": c["id"], "content": result})
            if tools.submitted:
                break
        return Reply(text)

    def _start(self, agent):
        if not agent.messages:
            agent.messages.append({"role": "system", "content": agent.system})

    def _chat(self, messages, tools=None, schema=None, label=""):
        body = {"model": self.a.model, "messages": messages}
        if self.a.effort:
            body["reasoning_effort"] = self.a.effort  # reasoning models: minimal / low / medium / high
        if tools:
            body["tools"] = tools
        if schema and self.schema_ok:
            body["response_format"] = {"type": "json_schema", "json_schema": {"name": "reply", "schema": schema, "strict": True}}
        elif schema:
            body["messages"] = messages + [{"role": "system", "content": "Reply with one JSON object matching this schema, and nothing else:\n" + json.dumps(schema)}]
        err = ""
        for attempt in range(7):
            req = urllib.request.Request(f"{self.base}/chat/completions", data=json.dumps(body).encode(), headers={
                "content-type": "application/json", **({"authorization": f"Bearer {self.key}"} if self.key else {})})
            try:
                with urllib.request.urlopen(req, timeout=900) as r:
                    resp = json.loads(r.read())
                u = resp.get("usage") or {}
                cached = (u.get("prompt_tokens_details") or {}).get("cached_tokens") or 0
                self.usage.add(turns=1, input_tokens=(u.get("prompt_tokens") or 0) - cached, cache_read_input_tokens=cached,
                               output_tokens=u.get("completion_tokens"))
                return resp["choices"][0]["message"]
            except urllib.error.HTTPError as e:
                err = f"HTTP {e.code}: {e.read()[:500]!r}"
                if e.code == 400 and schema and self.schema_ok and "response_format" in err:
                    self.schema_ok = False  # the server does not do JSON-schema replies: ask for JSON instead
                    return self._chat(messages, tools, schema, label)
                if e.code in (401, 403, 404):
                    break
            except (urllib.error.URLError, TimeoutError, OSError) as e:
                err = str(e)
            self.retry_wait(attempt, label, err)
        raise RuntimeError(f"api failed ({label}): {err}")

    def save_transcripts(self, agents, dest):
        for label, agent in agents.items():
            if agent.messages:
                dest.mkdir(parents=True, exist_ok=True)
                (dest / f"{label}.json").write_text(json.dumps(agent.messages, indent=1, ensure_ascii=False))


BACKENDS = {"claude": ClaudeCLI, "codex": CodexCLI, "api": OpenAIAPI}
