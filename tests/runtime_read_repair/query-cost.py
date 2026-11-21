"""Read-only finite comparison of old per-Agent unread scans and shared batch scan."""
import argparse
import json
from pathlib import Path
import sqlite3
import time

p = argparse.ArgumentParser()
p.add_argument('database')
p.add_argument('output')
a = p.parse_args()
db = sqlite3.connect('file:' + str(Path(a.database).resolve(strict=True)) + '?mode=ro', uri=True)
rows = db.execute("SELECT name,last_event_id FROM instances WHERE COALESCE(origin_device_id,'')='' ORDER BY name LIMIT 100").fetchall()
recipients = dict(rows)
report = {'database': str(Path(a.database).resolve()), 'recipients': len(rows), 'events': db.execute('SELECT COUNT(*) FROM events').fetchone()[0], 'boundary': 'Python SQLite read-only query-shape measurement; not native HTTP latency'}
try:
    started = time.monotonic()
    db.set_progress_handler(lambda: int(time.monotonic() - started > 30), 10000)
    old = {}
    try:
        for name, cursor in rows:
            old[name] = db.execute("SELECT COUNT(*) FROM events WHERE id>? AND type='message' AND EXISTS(SELECT 1 FROM json_each(json_extract(data,'$.delivered_to')) WHERE value=?)", (cursor, name)).fetchone()[0]
        report['old_ms'] = round((time.monotonic() - started) * 1000)
    except sqlite3.OperationalError as error:
        report['old_ms'] = round((time.monotonic() - started) * 1000)
        report['old_incomplete'] = str(error)
    report['old_recipients_completed'] = len(old)
    started = time.monotonic()
    counts = {name: 0 for name in recipients}
    for event_id, delivered in db.execute("SELECT id,json_extract(data,'$.delivered_to') FROM events WHERE type='message' AND id>? ORDER BY id", (min(recipients.values()),)):
        values = json.loads(delivered) if delivered else []
        if not isinstance(values, list):
            continue
        for name in set(value for value in values if isinstance(value, str)):
            if name in recipients and event_id > recipients[name]:
                counts[name] += 1
    report['batch_ms'] = round((time.monotonic() - started) * 1000)
    report['completed_counts_match'] = all(counts[name] == count for name, count in old.items())
    report['old_message_scans'] = len(rows)
    report['batch_message_scans'] = 1
finally:
    db.close()
Path(a.output).write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps(report))
