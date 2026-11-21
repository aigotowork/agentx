"""Extend existing isolated fixtures with one finite generated-hook Codex substitute."""
from pathlib import Path
import shutil
import sys

sys.path.insert(0, str(Path(__file__).resolve().parents[1] / "office_management"))
from office_fixture import OfficeFixture, wait_for


class LiveFixture(OfficeFixture):
    def __init__(self, candidate, evidence):
        super().__init__(candidate, evidence)
        # Extend only the private staged fixture to consume the native format
        # for a deliberate message with intent:null (Inspector's supported send).
        # The upstream fixture deliberately handled request/inform only.
        qoder = self.root / "bin/qoder"
        original = qoder.read_text()
        pattern = r"\[(?:request|inform)(?::[^\]\s]+)? #(\d+)\] bigboss"
        expanded = r"\[(?:(?:request|inform)(?::[^\]\s]+)?|new message) #(\d+)\] bigboss"
        assert original.count(pattern) == 1, "upstream fixture message recipe changed"
        qoder.write_text(original.replace(pattern, expanded))
        source = Path(__file__).with_name("codex_provider.py")
        shutil.copyfile(source, self.root / "bin/codex")
        (self.root / "bin/codex").chmod(0o755)

    def environment(self, name):
        environment = super().environment(name)
        # Private provider home; never use installed/shared Codex roots.
        environment["CODEX_HOME"] = str(self.root / name / "home/.codex")
        return environment
