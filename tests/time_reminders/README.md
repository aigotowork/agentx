---
scope: Linux CLI acceptance for native one-shot time reminders
owner: Events integration
links: [../../.controller/time-reminders/implementation.task.md]
---

Run with Python 3.8+ and an already built Linux candidate:

```sh
python3 tests/time_reminders/acceptance.py /absolute/path/to/candidate/agentx
```

Keep that executable fixed throughout the run. The runner prints the resolved path and version, timing measurements, and a final PASS only after all checks and cleanup succeed. A failure exits nonzero and prints the failing assertion or command and any daemon log tail.

The runner creates its own private Scope, home, working directory and temporary directories under `/tmp`. It boots and stops only its own foreground daemon, pins the Scope's startup lock to prevent implicit spawning, and removes the fixture at exit. Help is tested against a separate nonexistent Scope before the daemon starts. It does not require a Provider or use a pre-existing Scope, installed default, package recipe, Relay setup or network service.

| Case | Evidence collected by the script |
| --- | --- |
| CLI and authorization | Normal and recursive help; valid durations and conflicts; implicit caller and exact `--for` target; distinct IDs for repeated commands; list, history, JSON show and cancel. Creator and original target can cancel; another Agent and an anonymous caller cannot. |
| Scheduling | Timing comes from daemon-persisted `completed_at_ms` and `due_at_ms`; a read-only SQLite probe avoids sending requests in the quiet phase. The load phase uses four concurrent event query loops and checks reply timestamps around the due window. Both phases require 0–2,000 ms from due to persisted completion. |
| Recovery and delivery | Overdue processing after orderly daemon stop and restart; canceled, stopped and replaced targets produce no delivery; exact system event and original inbox delivery; unchanged event IDs and one event per reminder after a second restart. The replacement starts with cursor zero and must see the old ordinary message without the old fenced reminder. |
| Legacy | A structured once subscription ignores reminders, fires on an ordinary direct message, then disappears; a positional SQL subscription persists across restarts and can be removed normally. |

Identity fixture writes are limited to the disposable `instances` table while the daemon is stopped. Reminders, subscriptions, deliveries and mailbox reads use the actual CLI/native service. The quiet phase makes no fixture CLI calls while waiting for due processing; the daemon may still run its own background tasks. Load reply cadence demonstrates sustained external native traffic, without proving continuous writer queue saturation. The restart cases do not inject a crash between commit and wake or an uncertain create reply. The script does not establish Provider wake/read behavior, every internal unread projection, migration from v29, protocol compatibility, or live adoption.
