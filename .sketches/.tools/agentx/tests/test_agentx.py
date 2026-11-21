import json
from contextlib import redirect_stderr, redirect_stdout
from io import StringIO
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import catalog
import cli
import runtime
import sessions
from storage import AgentxError, read_document, read_yaml, write_yaml



class Workspace(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name).resolve()
        self.scope = self.root / "apps/web"
        self.scope.mkdir(parents=True)
        self.task = self.root / "work.md"
        self.body = "\r\n# Isolated test task\r\n\r\nKeep *this* body exactly.\r\n"
        self.task.write_bytes(("\ufeff---\r\nschema: task\r\nstatus: planned # preserve this comment\r\n"
                               "custom: {label: '001', flag: false}\r\n---\r\n" + self.body).encode())
        self.write(".agent-guides/roles/developer.md", "---\nschema: role\nrole: developer\nskills: [inspect]\n---\nOwn the bound task.\n")
        self.skill(".agent-guides/skills/inspect", "inspect", "Inspect project files.")
        self.environment = patch.dict(os.environ, {}, clear=True)
        self.environment.start()
        self.addCleanup(self.environment.stop)

    def write(self, relative, content):
        path = self.root / relative
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content)
        return path

    def skill(self, relative, name, description):
        return self.write(f"{relative}/SKILL.md", f"---\nname: {name}\ndescription: {description}\n---\nInstructions.\n")

    def create(self, **options):
        return sessions.create(self.root, self.scope, role_ref="developer", task=self.task, **options)

    def bind_agentx(self, directory, handle=None):
        _, data = sessions.load(directory)
        data["agentx"] = handle or {"name": "owned-agent", "session_id": "native-thread"}
        write_yaml(directory / "session.yaml", data)
        return data["agentx"]


class BindingTests(Workspace):
    def test_local_definitions_shadow_ancestors_and_unbound_skills_stay_out(self):
        local = self.skill("apps/web/.agent-guides/skills/inspect", "inspect", "Inspect this module.")
        self.skill(".agent-guides/skills/unbound", "unbound", "Should not be bound.")
        directory, record = self.create(extra_skills=["inspect"])
        self.assertEqual([item["name"] for item in record["skills"]], ["inspect"])
        self.assertEqual(record["skills"][0]["path"], str(local))
        self.assertEqual(record["skills"][0]["via"], ["role", "launch"])
        self.assertTrue((directory / "session.yaml").is_file())
        self.assertEqual(sorted(path.name for path in directory.iterdir()), ["session.yaml"])

    def test_missing_skill_has_no_partially_created_session(self):
        with self.assertRaises(AgentxError):
            self.create(extra_skills=["missing"])
        self.assertFalse((self.scope / ".agentx/sessions").exists())

    def test_conflicting_skill_names_are_rejected(self):
        duplicate = self.skill("other", "inspect", "Another implementation.")
        with self.assertRaisesRegex(AgentxError, "share name"):
            self.create(extra_skills=[str(duplicate)])

    def test_context_identity_survives_cwd_and_role_source_changes(self):
        directory, _ = self.create()
        role = self.root / ".agent-guides/roles/developer.md"
        role.write_text(role.read_text().replace("Own the bound task.", "Different role now."))
        os.environ["AGENTX_SESSION_DIR"] = str(directory)
        original = Path.cwd()
        try:
            os.chdir(self.root)
            value = sessions.context()
        finally:
            os.chdir(original)
        self.assertIn("Own the bound task.", value["role"]["instructions"])
        self.assertEqual(value["scope_dir"], str(self.scope))

    def test_directory_lookup_does_not_visit_siblings_or_escape_root(self):
        self.write("apps/api/.agent-guides/roles/api.md", "---\nschema: role\nrole: api\n---\nAPI only.\n")
        with self.assertRaises(AgentxError):
            catalog.load_role("api", self.scope, self.root)
        with self.assertRaises(AgentxError):
            list(catalog.scope_chain(self.root.parent, self.root))

    def test_configuration_merges_maps_and_replaces_lists(self):
        self.write(".agentx/config.yaml", "runtime:\n  env: {FIRST: one, SECOND: two}\n  codex_args: [--model, first]\n")
        self.write("apps/web/.agentx/config.yaml", "runtime:\n  env: {SECOND: override}\n  codex_args: [--model, second]\n")
        _, record = self.create(env=["FIRST=explicit"])
        self.assertEqual(record["env"], {"FIRST": "explicit", "SECOND": "override"})
        self.assertEqual(record["codex_args"], ["--model", "second"])


class ResourceTests(Workspace):
    def test_status_reads_current_task_metadata_without_body_or_writes(self):
        directory, _ = self.create()
        for state in ["planned", "reviewed-externally", "completed"]:
            document = read_document(self.task)
            document.metadata.update(status=state, progress="Edited by the agent", next_action="Read the result")
            document.save()
            before = {path: path.read_bytes() for path in [self.task, directory / "session.yaml"]}
            status = sessions.status(directory)
            context = sessions.context(directory)
            self.assertEqual(status["task"]["metadata"]["status"], state)
            self.assertEqual(status["task"]["metadata"]["progress"], "Edited by the agent")
            self.assertNotIn("body", status["task"])
            self.assertEqual(context["task"]["body"], self.body)
            for path, content in before.items():
                self.assertEqual(path.read_bytes(), content)
        saved = read_yaml(directory / "session.yaml")
        self.assertNotIn("task_status", saved)
        self.assertNotIn("agentx_session", read_document(self.task).metadata)

    def test_status_reports_missing_resources_and_keeps_the_retained_role(self):
        directory, record = self.create()
        self.task.unlink()
        Path(record["role"]["path"]).unlink()
        Path(record["skills"][0]["path"]).unlink()
        value = sessions.status(directory)
        self.assertFalse(value["task"]["available"])
        self.assertFalse(value["role"]["available"])
        self.assertFalse(value["skills"][0]["available"])
        context = sessions.context(directory)
        self.assertIn("Own the bound task.", context["role"]["instructions"])
        self.assertIn("Unavailable:", sessions.context_text(context))

    def test_invalid_task_frontmatter_does_not_hide_other_bindings(self):
        directory, _ = self.create()
        self.task.write_text("No frontmatter here")
        before = self.task.read_bytes()
        value = sessions.status(directory)
        self.assertFalse(value["task"]["available"])
        self.assertIn("frontmatter", value["task"]["diagnostic"])
        self.assertEqual(value["role"]["name"], "developer")
        self.assertEqual(self.task.read_bytes(), before)

    def test_task_schema_and_lifecycle_belong_to_the_task_owner(self):
        document = read_document(self.task)
        document.metadata.update(schema="external-task", status="review-pending")
        document.save()
        before = self.task.read_bytes()
        directory, _ = self.create()
        self.assertEqual(sessions.status(directory)["task"]["metadata"]["status"], "review-pending")
        self.assertEqual(self.task.read_bytes(), before)

    def test_unbound_status_and_context_are_usable_without_a_task(self):
        directory, _ = sessions.create(self.root, self.scope)
        self.assertIsNone(sessions.status(directory)["task"])
        self.assertIsNone(sessions.context(directory)["task"])
        self.assertIn(directory.name, sessions.context_text(sessions.context(directory)))
        self.assertEqual(sorted(path.name for path in directory.iterdir()), ["session.yaml"])


class DiscoveryTests(Workspace):
    def test_discovery_matches_directory_shadowing_and_does_not_visit_siblings(self):
        local = self.skill("apps/web/.agent-guides/local/skills/inspect", "inspect", "Inspect this module.")
        self.skill("apps/api/.agent-guides/skills/api-only", "api-only", "Sibling module.")
        value = catalog.discover(self.scope, self.root)
        self.assertEqual([role["name"] for role in value["roles"]], ["developer"])
        self.assertEqual(value["roles"][0]["skills"], ["inspect"])
        self.assertEqual([item["path"] for item in value["skills"]], [str(local)])

    def test_discovery_exposes_nested_skills_by_bindable_source_path(self):
        nested = self.skill(".agent-guides/skills/group/review", "review", "Review work.")
        value = catalog.discover(self.scope, self.root)
        item = next(item for item in value["skills"] if item["name"] == "review")
        self.assertEqual(item["path"], str(nested))
        directory, record = self.create(extra_skills=[item["path"]])
        self.assertIn("review", [skill["name"] for skill in record["skills"]])


class CliTests(Workspace):
    def invoke(self, arguments):
        output = StringIO()
        with patch.object(sys, "argv", ["agentx", *arguments]), redirect_stdout(output):
            cli.main()
        return output.getvalue()

    def test_new_session_context_is_readable_before_launch(self):
        directory, _ = self.create()
        output = self.invoke(["mycontext", "--session", str(directory)])
        self.assertIn(directory.name, output)
        self.assertIn("Own the bound task.", output)

    def test_common_options_work_before_and_after_subcommands(self):
        directory, _ = self.create()
        for arguments in [["--format", "yaml", "mycontext"], ["mycontext", "--format", "yaml"],
                          ["--format", "text", "mycontext", "--format", "yaml"]]:
            output = self.invoke([*arguments, "--session", directory.name, "--dir", str(self.scope)])
            self.assertEqual(cli.yaml_parser().load(output)["session"], directory.name)

    def test_short_id_selects_only_its_explicit_directory(self):
        directory, _ = self.create()
        output = self.invoke(["status", "--session", directory.name, "--dir", str(self.scope)])
        self.assertEqual(cli.yaml_parser().load(output)["session_dir"], str(directory))
        with self.assertRaises(AgentxError):
            sessions.load(sessions.session_directory(directory.name, self.root))

    def test_root_option_after_create_does_not_capture_forwarded_codex_options(self):
        with patch("runtime.start", side_effect=lambda directory, **kwargs: {"session_dir": str(directory), "outcome": "prepared"}):
            output = self.invoke(["launch", "--dir", str(self.scope), "--root", str(self.root),
                                  "--role", "developer", "--", "--model", "chosen-model", "--config", "model_verbosity=low"])
        directory = cli.yaml_parser().load(output)["session_dir"]
        _, record = sessions.load(directory)
        self.assertEqual(record["codex_args"], ["--model", "chosen-model", "--config", "model_verbosity=low"])

    def test_list_and_context_read_current_agentx_state(self):
        directory, _ = self.create()
        self.bind_agentx(directory)
        active = {"name": "owned-agent", "session_id": "native-thread", "status": "blocked", "stopped": False}
        stopped = {"name": "owned-agent", "session_id": "native-thread", "stopped": True}
        with patch("runtime.observe", side_effect=[active, stopped]):
            listing = cli.yaml_parser().load(self.invoke(["discover", "--root", str(self.root), "--dir", str(self.scope)]))
            context = cli.yaml_parser().load(self.invoke(["mycontext", "--session", str(directory), "--format", "yaml"]))
        self.assertEqual(listing["agents"][0]["runtime"]["observation"], active)
        self.assertEqual(context["runtime"]["observation"], stopped)
        self.assertEqual(context["task"]["metadata"]["status"], "planned")
        self.assertNotIn("status", listing["agents"][0])

    def test_unavailable_agentx_keeps_context_usable_without_claiming_runtime_state(self):
        directory, _ = self.create()
        self.bind_agentx(directory)
        with patch("runtime.subprocess.run", side_effect=FileNotFoundError("agentx is unavailable")) as process:
            value = cli.yaml_parser().load(self.invoke(["mycontext", "--session", str(directory), "--format", "yaml"]))
        self.assertIsNone(value["runtime"]["observation"])
        self.assertIn("agentx is unavailable", value["runtime"]["diagnostic"])
        self.assertNotIn("status", value["agentx"])
        self.assertIn("Own the bound task.", value["role"]["instructions"])
        self.assertEqual(process.call_count, 1)

    def test_agent_uses_injected_identity_without_target_or_caller_arguments(self):
        directory, _ = self.create()
        handle = self.bind_agentx(directory)
        os.environ.update(AGENTX_SESSION_DIR=str(directory), AGENTX_INSTANCE_NAME="vali")
        observed = {**handle, "status": "listening"}
        original = Path.cwd()
        try:
            os.chdir("/tmp")
            with patch("runtime.subprocess.run", return_value=subprocess.CompletedProcess([], 0, json.dumps(observed), "")) as process:
                context = cli.yaml_parser().load(self.invoke(["mycontext", "--format", "yaml"]))
                status = cli.yaml_parser().load(self.invoke(["status"]))
            self.assertEqual(context["session"], directory.name)
            self.assertEqual(status["session"], directory.name)
            self.assertEqual(context["task"]["path"], str(self.task))
            self.assertTrue(all(call.args[0][-2:] == ["--name", "vali"] for call in process.call_args_list))
        finally:
            os.chdir(original)

    def test_resume_uses_injected_session_when_target_is_omitted(self):
        directory, _ = self.create()
        self.bind_agentx(directory)
        os.environ.update(AGENTX_SESSION_DIR=str(directory), AGENTX_INSTANCE_NAME="vali")
        reply = subprocess.CompletedProcess([], 0, json.dumps({"names": ["owned-agent"]}), "")
        with patch("runtime.subprocess.run", return_value=reply) as process, patch("runtime.observe", return_value=None):
            value = cli.yaml_parser().load(self.invoke(["resume"]))
        self.assertEqual(value["session"], directory.name)
        self.assertEqual(process.call_args.args[0][1:3], ["r", "native-thread"])
        self.assertEqual(process.call_args.args[0][-2:], ["--name", "vali"])

    def test_explicit_target_overrides_self_without_guessing_the_latest_session(self):
        own, _ = self.create()
        other, _ = self.create()
        os.environ["AGENTX_SESSION_DIR"] = str(own)
        selected = cli.yaml_parser().load(self.invoke(["status", "--session", other.name, "--dir", str(self.scope)]))
        self.assertEqual(selected["session"], other.name)
        del os.environ["AGENTX_SESSION_DIR"]
        with self.assertRaisesRegex(AgentxError, "No session identity"):
            cli.dispatch(cli.parser().parse_args(["status"]))

    def test_task_commands_are_not_part_of_the_public_interface(self):
        with redirect_stderr(StringIO()):
            with self.assertRaises(SystemExit) as error:
                cli.parser().parse_args(["task", "start"])
        self.assertEqual(error.exception.code, 2)

    def test_unknown_launch_result_uses_the_command_exit_code(self):
        output = StringIO()
        arguments = ["agentx", "launch", "--root", str(self.root),
                     "--dir", str(self.scope)]
        with patch.object(sys, "argv", arguments), patch("runtime.subprocess.run", side_effect=subprocess.TimeoutExpired("agentx", 1)), redirect_stdout(output):
            with self.assertRaises(SystemExit) as error:
                cli.main()
        self.assertEqual(error.exception.code, 3)
        self.assertEqual(cli.yaml_parser().load(output.getvalue())["outcome"], "unknown")


class RuntimeTests(Workspace):
    def test_external_terminal_delegates_identity_resolution_to_agentx(self):
        reply = subprocess.CompletedProcess([], 0, "[]", "")
        with patch("runtime.subprocess.run", return_value=reply) as process:
            runtime.invoke(["list", "owned-agent", "--json"], None, 1)
        self.assertEqual(process.call_args.args[0], ["agentx", "list", "owned-agent", "--json"])

    def test_environment_and_prompt_are_derived_without_creating_files(self):
        directory, record = self.create(env=["EXAMPLE=value with spaces and $(literal)"])
        arguments, env = runtime.command(directory, record)
        self.assertEqual(env["AGENTX_SESSION_DIR"], str(directory))
        self.assertEqual(env["EXAMPLE"], "value with spaces and $(literal)")
        self.assertTrue(env["PATH"].startswith(str(sessions.TOOL_DIR)))
        instructions = arguments[arguments.index("--agentx-system-prompt") + 1]
        self.assertIn("Own the bound task.", instructions)
        self.assertNotIn("agentx task", instructions)
        self.assertEqual(sorted(path.name for path in directory.iterdir()), ["session.yaml"])
        self.assertEqual({key for key in env if key.startswith("AGENTX_")}, {"AGENTX_SESSION_DIR", "AGENTX_ROOT"})

    def test_launch_saves_only_native_identity_in_the_session(self):
        for response in [{"names": ["owned-agent"]}, {"instances": [{"name": "owned-agent", "status": "ready"}]}]:
            with self.subTest(response=response):
                directory, _ = self.create()
                observed = {"name": "owned-agent", "session_id": "thread-one", "status": "listening"}
                replies = [subprocess.CompletedProcess([], 0, json.dumps(response), ""),
                           subprocess.CompletedProcess([], 0, json.dumps(observed), "")]
                with patch("runtime.subprocess.run", side_effect=replies) as process:
                    value = runtime.start(directory, "vali")
                self.assertEqual(value["outcome"], "submitted")
                self.assertEqual(value["runtime"]["observation"]["status"], "listening")
                saved = sessions.load(directory)[1]
                self.assertEqual(saved["agentx"], {"name": "owned-agent", "session_id": "thread-one"})
                self.assertNotIn("outcome", saved)
                self.assertNotIn("response", saved)
                self.assertNotIn("status", saved)
                self.assertEqual(sorted(path.name for path in directory.iterdir()), ["session.yaml"])
                self.assertNotIn("shell", process.call_args_list[0].kwargs)

    def test_preview_does_not_bind_an_unlaunched_identity(self):
        directory, _ = self.create()
        before = (directory / "session.yaml").read_bytes()
        reply = subprocess.CompletedProcess([], 0, json.dumps({"preview": True, "names": ["preview-name"]}), "")
        with patch("runtime.subprocess.run", return_value=reply) as process:
            value = runtime.start(directory, "vali")
        self.assertEqual(value["outcome"], "prepared")
        self.assertIsNone(value["agentx"])
        self.assertEqual((directory / "session.yaml").read_bytes(), before)
        self.assertEqual(process.call_count, 1)

    def test_lost_launch_result_is_returned_without_automatic_retry_or_local_state(self):
        directory, _ = self.create()
        before = (directory / "session.yaml").read_bytes()
        with patch("runtime.subprocess.run", side_effect=subprocess.TimeoutExpired("agentx", 1)) as process:
            value = runtime.start(directory, "vali", 1)
        self.assertEqual(value["outcome"], "unknown")
        self.assertIn("inspect agentx", value["diagnostic"])
        self.assertEqual(process.call_count, 1)
        self.assertEqual((directory / "session.yaml").read_bytes(), before)
        self.assertEqual(sorted(path.name for path in directory.iterdir()), ["session.yaml"])

    def test_resume_delegates_admission_to_agentx_and_preserves_task_state(self):
        directory, _ = self.create()
        handle = self.bind_agentx(directory)
        document = read_document(self.task)
        document.metadata.update(status="completed", result="Finished")
        document.save()
        task_before = self.task.read_bytes()
        before = (directory / "session.yaml").read_bytes()
        reply = subprocess.CompletedProcess([], 1, "", "Native runtime is already active")
        with patch("runtime.subprocess.run", return_value=reply) as process:
            def after_dispatch(*args):
                self.assertTrue(process.called)
                return {**handle, "status": "active", "stopped": False}
            with patch("runtime.observe", side_effect=after_dispatch):
                value = runtime.start(directory, "vali")
        self.assertEqual(process.call_args.args[0][1:3], ["r", "native-thread"])
        self.assertNotIn("--headless", process.call_args.args[0])
        self.assertEqual(value["session"], directory.name)
        self.assertEqual(value["outcome"], "failed")
        self.assertEqual(value["diagnostic"], "Native runtime is already active")
        self.assertEqual(read_document(self.task).metadata["status"], "completed")
        self.assertEqual(self.task.read_bytes(), task_before)
        self.assertEqual((directory / "session.yaml").read_bytes(), before)

    def test_failed_resume_keeps_the_original_conversation_reference(self):
        directory, _ = self.create()
        handle = self.bind_agentx(directory)
        reply = subprocess.CompletedProcess([], 1, "", "Explicit launch failure")
        with patch("runtime.observe", return_value={**handle, "stopped": True}), patch("runtime.subprocess.run", return_value=reply) as process:
            runtime.start(directory, "vali")
            runtime.start(directory, "vali")
        self.assertEqual([call.args[0][1:3] for call in process.call_args_list],
                         [["r", "native-thread"], ["r", "native-thread"]])
        self.assertEqual(sessions.load(directory)[1]["agentx"], handle)
        self.assertEqual(sorted(path.name for path in directory.iterdir()), ["session.yaml"])

    def test_successful_resume_keeps_session_edits_made_while_waiting_for_agentx(self):
        directory, _ = sessions.create(self.root, self.scope)
        self.bind_agentx(directory)
        def reply(arguments, sender, timeout, env=None):
            self.assertEqual(arguments[:2], ["r", "native-thread"])
            _, edited = sessions.load(directory)
            edited["custom_label"] = "Saved while waiting"
            write_yaml(directory / "session.yaml", edited)
            return subprocess.CompletedProcess([], 0, json.dumps({"names": ["resumed-agent"]}), "")
        observed = {"name": "resumed-agent", "session_id": "native-thread", "status": "active", "stopped": False}
        with patch("runtime.invoke", side_effect=reply), patch("runtime.observe", return_value=observed):
            value = runtime.start(directory, "vali")
        saved = sessions.load(directory)[1]
        self.assertEqual(value["outcome"], "submitted")
        self.assertEqual(value["session"], directory.name)
        self.assertEqual(saved["agentx"], {"name": "resumed-agent", "session_id": "native-thread"})
        self.assertEqual(saved["custom_label"], "Saved while waiting")

    def test_external_stop_is_visible_without_rewriting_session_or_task(self):
        directory, _ = self.create()
        handle = self.bind_agentx(directory)
        before = {path: path.read_bytes() for path in [directory / "session.yaml", self.task]}
        active = {**handle, "status": "active", "stopped": False}
        stopped = {**handle, "stopped": True}
        with patch("runtime.observe", side_effect=[active, stopped]):
            first = cli.resource_status(directory, 1)
            second = cli.resource_status(directory, 1)
        self.assertEqual(first["runtime"]["observation"], active)
        self.assertEqual(second["runtime"]["observation"], stopped)
        self.assertEqual(second["runtime"]["source"], "agentx")
        self.assertIn("observed_at", second["runtime"])
        for path, content in before.items():
            self.assertEqual(path.read_bytes(), content)

    def test_unavailable_agentx_does_not_invent_a_runtime_state(self):
        directory, _ = self.create()
        self.bind_agentx(directory)
        before = (directory / "session.yaml").read_bytes()
        with patch("runtime.observe", side_effect=subprocess.TimeoutExpired("agentx", 1)):
            value = cli.resource_status(directory, 1)
        self.assertIsNone(value["runtime"]["observation"])
        self.assertIn("timed out", value["runtime"]["diagnostic"])
        self.assertEqual((directory / "session.yaml").read_bytes(), before)

    def test_observation_timeout_does_not_change_successful_submission(self):
        directory, _ = self.create()
        reply = subprocess.CompletedProcess([], 0, json.dumps({"names": ["owned-agent"]}), "")
        with patch("runtime.subprocess.run", side_effect=[reply, subprocess.TimeoutExpired("agentx", 1)]):
            value = runtime.start(directory, "vali")
        self.assertEqual(value["outcome"], "submitted")
        self.assertIsNone(value["runtime"]["observation"])
        self.assertEqual(sessions.load(directory)[1]["agentx"], {"name": "owned-agent"})

    def test_live_identity_must_match_the_bound_conversation(self):
        directory, _ = self.create()
        self.bind_agentx(directory)
        reused = {"name": "owned-agent", "session_id": "other-thread", "status": "listening"}
        replies = [subprocess.CompletedProcess([], 0, json.dumps(reused), ""),
                   subprocess.CompletedProcess([], 0, "[]", "")]
        with patch("runtime.subprocess.run", side_effect=replies):
            value = cli.resource_status(directory, 1)
        self.assertIsNone(value["runtime"]["observation"])
        self.assertEqual(value["agentx"]["session_id"], "native-thread")

    def test_stopped_lookup_comes_from_agentx_after_active_lookup_fails(self):
        handle = {"name": "owned-agent", "session_id": "native-thread"}
        replies = [subprocess.CompletedProcess([], 1, "", "Agent stopped"),
                   subprocess.CompletedProcess([], 0, json.dumps(handle), "")]
        with patch("runtime.subprocess.run", side_effect=replies) as process:
            observation = runtime.observe(handle, "vali", 1)
        self.assertTrue(observation["stopped"])
        self.assertEqual(process.call_args_list[1].args[0],
                         ["agentx", "list", "--stopped", "owned-agent", "--json", "--name", "vali"])



if __name__ == "__main__":
    unittest.main()
