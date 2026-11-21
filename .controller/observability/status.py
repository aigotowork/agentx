#!/usr/bin/env python3
"""Print original observability work state; never persist a second status ledger.

Uses the existing local Python/PyYAML document tooling. Run from any directory.
Stage prerequisites are acceptance guards, separate from active preparation.
"""

from pathlib import Path
import sys

import yaml


BASE = Path(__file__).resolve().parent
STATUSES = ("active", "blocked", "planned", "completed", "cancelled")
LABELS = {"active": "进行中", "blocked": "受阻", "planned": "待做", "completed": "已完成", "cancelled": "已取消"}


def read_document(path):
    content = path.read_text(encoding="utf-8")
    if not content.startswith("---\n"):
        raise ValueError(f"Missing metadata: {path}")
    metadata = yaml.safe_load(content.split("---", 2)[1])
    if not isinstance(metadata, dict):
        raise ValueError(f"Invalid metadata: {path}")
    return metadata


def main():
    delivery = read_document(BASE / "implementation.md")
    stages = delivery["stages"]
    rows = []
    for stage, definition in stages.items():
        prerequisites = ", ".join(
            f"{item['stage']}={stages[item['stage']].get('status', 'undeclared')}"
            for item in definition.get("depends_on", [])
        ) or "无上游阶段"
        for relative in definition.get("tasks", []):
            path = (BASE / relative).resolve()
            task = read_document(path)
            state = task["status"]
            if state not in STATUSES:
                raise ValueError(f"Invalid status {state}: {path}")
            if state == "blocked" and not task.get("blocking_reason"):
                raise ValueError(f"Blocked without reason: {path}")
            rows.append((state, path, stage, prerequisites, task))
    followup = BASE / "lan-usability.md"
    if followup.exists():
        task = read_document(followup)
        rows.append((task["status"], followup, "LAN follow-up", "explicit user follow-up", task))
    print(f"实施 Delivery: {delivery['status']}；设计 Delivery: {read_document(BASE / 'delivery.md')['status']}")
    print("状态来自原始 Task；阶段前提用于验收，准备工作可以并行。\n")
    for state in STATUSES:
        selected = [row for row in rows if row[0] == state]
        print(f"{LABELS[state]} ({len(selected)})")
        for _, path, stage, prerequisites, task in selected:
            print(f"- {path.stem} | 阶段: {stage} | 前提: {prerequisites}")
            print(f"  责任: {task.get('agent', 'unassigned')} | 原始记录: {path}")
            if state == "blocked":
                print(f"  阻塞: {task['blocking_reason']}")
        print()


if __name__ == "__main__":
    try:
        main()
    except (KeyError, OSError, ValueError, yaml.YAMLError) as error:
        print(f"Cannot derive work state: {error}", file=sys.stderr)
        raise SystemExit(1)
