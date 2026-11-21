from dataclasses import dataclass
from datetime import datetime, timezone
from io import StringIO
import os
from pathlib import Path
import tempfile

from ruamel.yaml import YAML


class AgentxError(Exception):
    pass


def now():
    return datetime.now(timezone.utc).isoformat(timespec="seconds")


def yaml_parser():
    parser = YAML(typ="rt")
    parser.preserve_quotes = True
    parser.width = 120
    parser.indent(mapping=2, sequence=4, offset=2)
    return parser


def yaml_text(data):
    output = StringIO()
    yaml_parser().dump(data, output)
    return output.getvalue()


def mapping(text, source):
    try:
        value = yaml_parser().load(text)
    except Exception as error:
        raise AgentxError(f"Invalid YAML in {source}: {error}") from error
    if not isinstance(value, dict):
        raise AgentxError(f"Expected a YAML mapping in {source}")
    return value


def read_yaml(path):
    return mapping(Path(path).read_text(), path)


def atomic_write(path, text):
    path = Path(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    mode = path.stat().st_mode & 0o777 if path.exists() else 0o600
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", newline="") as stream:
            stream.write(text)
            stream.flush()
            os.fsync(stream.fileno())
        os.chmod(temporary, mode)
        os.replace(temporary, path)
    finally:
        if os.path.exists(temporary):
            os.unlink(temporary)


def write_yaml(path, data):
    atomic_write(path, yaml_text(data))


@dataclass
class Document:
    path: Path
    metadata: dict
    body: str
    newline: str
    bom: str
    closing: str

    def save(self):
        header = yaml_text(self.metadata).replace("\n", self.newline)
        atomic_write(self.path, f"{self.bom}---{self.newline}{header}{self.closing}{self.body}")


def read_document(path):
    path = Path(path)
    with path.open(newline="") as stream:
        text = stream.read()
    bom = "\ufeff" if text.startswith("\ufeff") else ""
    lines = text.removeprefix(bom).splitlines(keepends=True)
    if not lines or lines[0].strip() != "---":
        raise AgentxError(f"Missing YAML frontmatter: {path}")
    for end in range(1, len(lines)):
        if lines[end].strip() in {"---", "..."}:
            newline = "\r\n" if lines[0].endswith("\r\n") else "\n"
            return Document(path, mapping("".join(lines[1:end]), path), "".join(lines[end + 1:]),
                            newline, bom, lines[end])
    raise AgentxError(f"Unclosed YAML frontmatter: {path}")
