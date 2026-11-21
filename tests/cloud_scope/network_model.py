"""Entrypoint for the scripted local model inside an isolated test container."""
import json
import os
from pathlib import Path
import threading

from native import Model


class LocalModelFiles:
    candidate = Path("/usr/local/bin/agentx")

    @staticmethod
    def write(name, value):
        Path("/tmp", name).write_text(json.dumps(value))


model = Model(LocalModelFiles())
Path(os.environ["CODEX_HOME"]).mkdir(parents=True, exist_ok=True)
with model:
    model.configure(os.environ)
    Path("/tmp/model-ids.json").write_text(json.dumps([model.request_id, model.reply_id]))
    threading.Event().wait()
