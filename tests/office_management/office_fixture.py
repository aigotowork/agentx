"""Office scenarios composed over the preserved public-management fixture."""
import json
from pathlib import Path
import sys

MANAGEMENT = Path(__file__).resolve().parents[1] / "agent_management"
sys.path.insert(0, str(MANAGEMENT))
from fixture import Fixture as ManagementFixture, wait_for


class OfficeFixture(ManagementFixture):
    def layout(self, name):
        base = self.root / name
        project = base / "project"
        paths = {
            "project": project,
            "child": project / "child",
            "nested": project / "child/src",
            "empty": project / "empty-office",
            "bare": project / "bare-agentx",
            "explicit": base / "explicit-membership",
            "cwd": base / "working-directory",
            "outside": base / "outside-office",
            "invalid": base / "invalid-office",
        }
        for path in paths.values():
            path.mkdir(parents=True, exist_ok=True)
        for key in ["project", "child", "empty", "outside", "invalid"]:
            (paths[key] / ".agentx/roles").mkdir(parents=True, exist_ok=True)
        (paths["bare"] / ".agentx").mkdir()
        # These apparent offices must never be traversed during project discovery.
        for directory in ["node_modules", ".git", "target", ".agentx/private", "one/two/three/four"]:
            (project / directory / ".agentx/roles").mkdir(parents=True, exist_ok=True)
        (project / "outside-link").symlink_to(paths["outside"], target_is_directory=True)
        scope = base / "scope"
        self.role(scope / "roles", "reviewer", "global reviewer")
        self.role(scope / "roles", "global-only", "global-only post")
        self.role(project / ".agentx/roles", "reviewer", "parent reviewer")
        self.role(project / ".agentx/roles", "controller", "parent default", default=True)
        self.role(paths["child"] / ".agentx/roles", "reviewer", "child reviewer")
        self.role(paths["outside"] / ".agentx/roles", "dispatcher", "outside default", default=True)
        return paths

    def role(self, directory, name, description, default=False, profile="management-fixture"):
        directory = Path(directory)
        directory.mkdir(parents=True, exist_ok=True)
        path = directory / (name + ".toml")
        path.write_text(f'profile = {json.dumps(profile)}\ndescription = {json.dumps(description)}\n'
                        f'default = {str(default).lower()}\nprompt = "PRIVATE_OFFICE_PROMPT_NEVER_PUBLIC"\n')
        return path
