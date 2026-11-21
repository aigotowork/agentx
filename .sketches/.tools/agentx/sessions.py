import os
from pathlib import Path
import re
from uuid import uuid4

from ruamel.yaml.scalarstring import LiteralScalarString

from catalog import bind_skills, config_for, load_role, scope_chain, strings
from storage import AgentxError, now, read_document, read_yaml, write_yaml, yaml_text


TOOL_DIR = Path(__file__).resolve().parent


def environment(values):
    result = {}
    for value in values:
        key, separator, content = value.partition("=")
        if not separator or not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*", key):
            raise AgentxError(f"Expected NAME=VALUE environment input: {value}")
        if key.startswith("AGENTX_"):
            raise AgentxError(f"Environment name is reserved: {key}")
        result[key] = content
    return result


def create(root, scope, role_ref=None, extra_skills=(), task=None, env=(), runtime_args=()):
    root, scope = Path(root).resolve(), Path(scope).resolve()
    list(scope_chain(scope, root))
    config = config_for(scope, root)
    runtime = config.get("runtime", {})
    if not isinstance(runtime, dict):
        raise AgentxError("runtime configuration must be a mapping")
    role, role_skills = load_role(role_ref, scope, root)
    skills = bind_skills(role, role_skills, extra_skills, scope, root)
    task_path = Path(task).expanduser().resolve() if task else None
    if task_path:
        read_document(task_path)
    defaults = runtime.get("env", {})
    if not isinstance(defaults, dict) or any(not isinstance(k, str) or not isinstance(v, str) for k, v in defaults.items()):
        raise AgentxError("runtime.env must map names to string values")
    effective_env = environment([f"{key}={value}" for key, value in defaults.items()])
    effective_env.update(environment(env))
    args = list(runtime_args) or strings(runtime.get("codex_args", []), "runtime.codex_args")
    if args and args[0] == "--":
        args = args[1:]
    if role:
        role["instructions"] = LiteralScalarString(role["instructions"])
    identifier = f"s_{uuid4().hex[:12]}"
    directory = scope / ".agentx/sessions" / identifier
    directory.mkdir(parents=True, mode=0o700)
    data = {"schema": "agentx-session", "id": identifier, "created_at": now(),
            "root": str(root), "scope_dir": str(scope), "role": role, "skills": skills,
            "task": str(task_path) if task_path else None, "env": effective_env,
            "codex_args": args, "agentx": None}
    parent = os.environ.get("AGENTX_SESSION_DIR")
    if parent:
        data["parent_session"] = str(Path(parent).resolve())
    write_yaml(directory / "session.yaml", data)
    return directory, data


def session_directory(selector=None, scope=None):
    value = selector or os.environ.get("AGENTX_SESSION_DIR")
    if not value:
        raise AgentxError("No session identity; pass --session DIRECTORY or set AGENTX_SESSION_DIR")
    if selector is not None and re.fullmatch(r"s_[0-9a-f]{12}", str(value)):
        return (Path(scope or os.environ.get("AGENTX_DIR", os.getcwd())) / ".agentx/sessions" / value).resolve()
    return Path(value).expanduser().resolve()


def load(selector=None):
    directory = session_directory(selector)
    if not (directory / "session.yaml").is_file():
        raise AgentxError(f"No session record at {directory}; use `agentx discover --dir <owning-directory>`")
    data = read_yaml(directory / "session.yaml")
    if data.get("schema") != "agentx-session" or data.get("id") != directory.name:
        raise AgentxError(f"Invalid session record: {directory}")
    return directory, data


def bootstrap(directory, data):
    lines = [f"# agentx session {data['id']}", f"Scope: {data['scope_dir']}",
             f"Session record: {directory / 'session.yaml'}",
             "Run `agentx mycontext` at the start and after context compaction.",
             "Maintain progress directly in the original task or its owning domain documents.",
             "Read the applicable project and directory AGENTS.md entries."]
    if data["role"]:
        lines += [f"\n## Role: {data['role']['name']}", f"Source: {data['role']['path']}",
                  "Resolve relative references in the role instructions from that source directory.",
                  str(data["role"]["instructions"])]
    lines += ["\n## Explicitly bound skills",
              "Read a bound skill's SKILL.md when its workflow applies; follow its resource references."]
    lines.extend(f"- {item['name']}: {item['description']}\n  Source: {item['path']}" for item in data["skills"])
    if not data["skills"]:
        lines.append("No skills are bound to this session.")
    if data["task"]:
        lines += ["\n## Task", data["task"], "Read this task and maintain its YAML frontmatter; preserve its Markdown body."]
    return "\n".join(lines) + "\n"


def context(selector=None):
    directory, data = load(selector)
    guides = [str(path / "AGENTS.md") for path in reversed(list(scope_chain(data["scope_dir"], data["root"])))
              if (path / "AGENTS.md").is_file()]
    value = {"session": data["id"], "session_dir": str(directory), "scope_dir": data["scope_dir"],
             "role": data["role"], "skills": bound_skills(data), "agentx": data["agentx"], "guides": guides,
             "task": task_resource(data["task"], include_body=True),
             "feedback": [str(path) for path in sorted((Path(data["scope_dir"]) / ".inbox").glob("*.feedback.md"))]}
    return value


def task_resource(path, include_body=False):
    if not path:
        return None
    try:
        document = read_document(path)
        value = {"path": path, "available": True, "metadata": document.metadata}
        if include_body:
            value["body"] = document.body
        return value
    except (AgentxError, OSError) as error:
        return {"path": path, "available": False, "diagnostic": str(error)}


def bound_skills(data):
    return [{**item, "available": Path(item["path"]).is_file()} for item in data["skills"]]


def status(selector=None):
    directory, data = load(selector)
    role = data["role"]
    return {"session": data["id"], "session_dir": str(directory), "scope_dir": data["scope_dir"],
            "task": task_resource(data["task"]),
            "role": {"name": role["name"], "path": role["path"], "available": Path(role["path"]).is_file()} if role else None,
            "skills": bound_skills(data), "agentx": data["agentx"]}


def context_text(value):
    role = value["role"]
    lines = [f"# Session {value['session']}", f"Scope: {value['scope_dir']}",
             f"Record: {value['session_dir']}/session.yaml"]
    if value["agentx"]:
        lines.append(f"agentx identity: {yaml_text(value['agentx']).strip()}")
    runtime = value.get("runtime", {})
    observation = runtime.get("observation")
    if observation:
        lines.append(f"agentx runtime: {yaml_text(observation).strip()}\nObserved at: {runtime['observed_at']}")
    elif runtime:
        lines.append(f"agentx runtime: unavailable ({runtime['diagnostic']})")
    if role:
        lines.extend([f"\n## Role: {role['name']}", f"Source: {role['path']}", str(role["instructions"])])
    lines.append("\n## Bound skills")
    lines.extend(f"- {skill['name']}: {skill['description']}\n  {skill['path']}" for skill in value["skills"])
    lines.extend(["\n## Directory guidance", *value["guides"]])
    if value["task"]:
        task = value["task"]
        lines.extend(["\n## Current task", task["path"]])
        if task["available"]:
            body = task["body"]
            if len(body) > 12000:
                body = body[:12000] + f"\n[Continues in {task['path']}]\n"
            lines.extend(["```yaml", yaml_text(task["metadata"]).rstrip(), "```", body])
        else:
            lines.append(f"Unavailable: {task['diagnostic']}")
    if value["feedback"]:
        lines.extend(["\n## Feedback sources", *value["feedback"]])
    return "\n".join(lines).rstrip() + "\n"


def directories(scope):
    return [path.parent for path in sorted((Path(scope).resolve() / ".agentx/sessions").glob("*/session.yaml"))]
