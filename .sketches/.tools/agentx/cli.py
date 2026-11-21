import argparse
import os
import subprocess
import sys

import catalog
import runtime
import sessions
from storage import AgentxError, yaml_parser, yaml_text


def parser():
    common = argparse.ArgumentParser(add_help=False)
    common.add_argument("--root", default=argparse.SUPPRESS,
                        help="workspace boundary (default: AGENTX_ROOT or the tool's project)")
    common.add_argument("--format", choices=["yaml", "text"], default=argparse.SUPPRESS)
    root = argparse.ArgumentParser(prog="agentx", parents=[common],
                                   description="Discover roles, skills and agents; launch with bindings and recover context.")
    commands = root.add_subparsers(dest="command", required=True)
    discovery = commands.add_parser("discover", parents=[common], help="list available roles, skills and agents in a directory")
    add_directory_option(discovery)
    add_runtime_options(discovery)
    launch = commands.add_parser("launch", parents=[common], help="start a new agent with role, skill and task bindings")
    add_directory_option(launch)
    launch.add_argument("--role", default=os.environ.get("AGENTX_ROLE"))
    launch.add_argument("--skill", action="append", help="additional skill; repeat to add several")
    launch.add_argument("--task", default=os.environ.get("AGENTX_TASK"), help="original task file to read; agentx never writes it")
    launch.add_argument("--env", action="append", default=[], metavar="NAME=VALUE")
    add_runtime_options(launch)
    launch.add_argument("runtime_args", nargs=argparse.REMAINDER, help="Codex arguments after --")
    resume = commands.add_parser("resume", parents=[common], help="resume a bound agent through agentx")
    resume.add_argument("session_directory", nargs="?", help="session path or ID; otherwise AGENTX_SESSION_DIR")
    add_directory_option(resume)
    add_runtime_options(resume)
    for name, help_text in [("status", "read bound task status and resource availability"),
                            ("mycontext", "recover role instructions, skill references and current task context")]:
        command = commands.add_parser(name, parents=[common], help=help_text)
        command.add_argument("--session", help="explicit session path or ID; otherwise AGENTX_SESSION_DIR")
        add_directory_option(command)
        add_runtime_options(command)
    return root


def add_directory_option(command):
    command.add_argument("--dir", default=os.environ.get("AGENTX_DIR", os.getcwd()),
                         help="working directory or scope for an explicit Session ID (default: CWD)")


def add_runtime_options(command):
    command.add_argument("--timeout", type=float, default=os.environ.get("AGENTX_TIMEOUT", "45"))


def resource_status(selector, timeout):
    value = sessions.status(selector)
    value["runtime"] = runtime.inspect_runtime(value["agentx"], timeout=timeout)
    return value


def dispatch(args):
    if args.command == "discover":
        value = catalog.discover(args.dir, args.root)
        value["agents"] = [resource_status(directory, args.timeout) for directory in sessions.directories(args.dir)]
        return value, None
    if args.command == "launch":
        try:
            env_skills = yaml_parser().load(os.environ.get("AGENTX_SKILLS", "[]"))
        except Exception as error:
            raise AgentxError(f"Invalid AGENTX_SKILLS YAML: {error}") from error
        extra_skills = args.skill if args.skill is not None else catalog.strings(env_skills, "AGENTX_SKILLS")
        directory, _ = sessions.create(args.root, args.dir, args.role, extra_skills, args.task,
                                        args.env, args.runtime_args)
        return runtime.start(directory, timeout=args.timeout), None
    if args.command == "resume":
        return runtime.start(sessions.session_directory(args.session_directory, args.dir), timeout=args.timeout), None
    selector = sessions.session_directory(args.session, args.dir)
    if args.command == "status":
        return resource_status(selector, args.timeout), None
    value = sessions.context(selector)
    value["runtime"] = runtime.inspect_runtime(value["agentx"], timeout=args.timeout)
    return value, sessions.context_text(value)


def main():
    defaults = argparse.Namespace(root=os.environ.get("AGENTX_ROOT", str(sessions.TOOL_DIR.parents[1])),
                                  format=os.environ.get("AGENTX_FORMAT"))
    arguments = parser().parse_args(namespace=defaults)
    try:
        if arguments.timeout <= 0:
            raise AgentxError("--timeout must be positive")
        value, text = dispatch(arguments)
        if text is not None and arguments.format != "yaml":
            print(text, end="")
        else:
            print(yaml_text(value), end="")
        outcome = value.get("outcome") if isinstance(value, dict) else None
        if arguments.command in {"launch", "resume"} and outcome in {"failed", "unknown"}:
            raise SystemExit(1 if outcome == "failed" else 3)
    except (AgentxError, OSError, subprocess.SubprocessError) as error:
        print(f"agentx: {error}", file=sys.stderr)
        raise SystemExit(2) from error
