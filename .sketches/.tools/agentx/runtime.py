import json
import os
import subprocess

from sessions import TOOL_DIR, bootstrap, load
from storage import AgentxError, now, write_yaml


def decode_output(text):
    text = text or ""
    try:
        return json.loads(text)
    except ValueError:
        values = []
        for line in text.splitlines():
            try:
                values.append(json.loads(line))
            except ValueError:
                continue
        return values or {"output": text}


def records(value):
    if isinstance(value, dict):
        if isinstance(value.get("name"), str):
            yield value
        for item in value.values():
            yield from records(item)
    elif isinstance(value, list):
        for item in value:
            yield from records(item)


def handle_from_response(response, sender):
    if isinstance(response, dict) and isinstance(response.get("names"), list):
        names = [name for name in response["names"] if isinstance(name, str) and name != sender]
        if len(names) == 1:
            return {"name": names[0]}
    candidates = [item for item in records(response) if item.get("name") != sender]
    return {key: candidates[0][key] for key in ["name", "session_id"] if key in candidates[0]} if len(candidates) == 1 else {}


def invoke(arguments, sender, timeout, env=None):
    sender = sender or os.environ.get("AGENTX_INSTANCE_NAME")
    identity = ["--name", sender] if sender else []
    return subprocess.run(["agentx", *arguments, *identity], capture_output=True, text=True,
                          timeout=timeout, env=env)


def observe(handle, sender, timeout):
    name = handle.get("name")
    if not name:
        return None
    errors = []
    for stopped in [False, True]:
        arguments = ["list", *( ["--stopped"] if stopped else []), name, "--json"]
        reply = invoke(arguments, sender, timeout)
        if reply.returncode:
            errors.append(reply.stderr.strip() or reply.stdout.strip() or f"agentx exited {reply.returncode}")
            continue
        for record in records(decode_output(reply.stdout)):
            if (record.get("name") == name or
                    (handle.get("session_id") and record.get("session_id") == handle["session_id"])):
                if handle.get("session_id") and record.get("session_id") != handle["session_id"]:
                    continue
                return {**{key: record[key] for key in ["name", "session_id", "status", "status_context", "status_detail", "directory"] if key in record},
                        "stopped": stopped}
    if errors:
        raise AgentxError("; ".join(errors))
    return None


def inspect_runtime(handle, sender=None, timeout=15):
    value = {"source": "agentx", "observation": None}
    if not handle or not handle.get("name"):
        value["diagnostic"] = "No launched agentx identity is recorded"
    else:
        try:
            value["observation"] = observe(handle, sender, timeout)
            if not value["observation"]:
                value["diagnostic"] = "agentx returned no matching runtime identity"
        except (AgentxError, OSError, subprocess.SubprocessError) as error:
            value["diagnostic"] = str(error)
        value["observed_at"] = now()
    return value


def command(directory, session, resume_target=None):
    child_env = dict(session["env"])
    path = child_env.get("PATH", os.environ.get("PATH", ""))
    child_env.update({"AGENTX_SESSION_DIR": str(directory),
                      "AGENTX_ROOT": session["root"], "PATH": f"{TOOL_DIR}{os.pathsep}{path}"})
    instructions = bootstrap(directory, session)
    prompt = "Run `agentx mycontext`, read the bound task if present, and continue within the role and task scope."
    base = (["r", resume_target, "--go"] if resume_target else ["1", "codex", "--go"])
    arguments = [*base, *([] if resume_target else ["--headless"]), "--json", "--dir", session["scope_dir"],
                 "--agentx-system-prompt", instructions, "--agentx-prompt", prompt,
                 *session["codex_args"], "-c", "skills.include_instructions=false"]
    for key, value in child_env.items():
        arguments += ["-c", f"shell_environment_policy.set.{key}={json.dumps(value, ensure_ascii=False)}"]
    return arguments, {**os.environ, **child_env}


def start(selector, sender=None, timeout=45):
    directory, session = load(selector)
    handle = session["agentx"] or {}
    target = handle.get("session_id") or handle.get("name")
    arguments, child_env = command(directory, session, target)
    result = {"session_dir": str(directory), "session": session["id"]}
    try:
        reply = invoke(arguments, sender, timeout, child_env)
        response = decode_output(reply.stdout)
        result.update(outcome="submitted" if reply.returncode in {0, 2} else "failed",
                      exit_code=reply.returncode, response=response)
        if reply.stderr.strip():
            result["diagnostic"] = reply.stderr.strip()
        if isinstance(response, dict) and response.get("preview") is True:
            result["outcome"] = "prepared"
        else:
            handle = {**handle, **handle_from_response(response, sender)}
            if not handle and reply.returncode in {0, 2}:
                result["outcome"] = "unknown"
    except subprocess.TimeoutExpired:
        result.update(outcome="unknown", diagnostic="agentx timed out; inspect agentx before retrying")
    except OSError as error:
        result.update(outcome="failed", diagnostic=str(error))
    result["runtime"] = inspect_runtime(handle, sender, timeout)
    if result["runtime"]["observation"]:
        observation = result["runtime"]["observation"]
        handle = {key: observation[key] for key in ["name", "session_id"] if key in observation}
    if handle and handle != session["agentx"]:
        _, session = load(directory)
        session["agentx"] = handle
        write_yaml(directory / "session.yaml", session)
    result["agentx"] = session["agentx"]
    return result


