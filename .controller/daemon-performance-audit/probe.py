#!/usr/bin/env python3
"""Compare candidate query shapes using only deterministic in-memory SQLite data."""

from collections import Counter
import json
from pathlib import Path
import re
import sqlite3


ROOT = Path(__file__).resolve().parents[2]


def source_sql(path, contains):
    source = (ROOT / path).read_text()
    return next(chunk for chunk in source.split('"') if chunk.startswith('SELECT ') and contains in chunk)


def measure(db, sql, arguments):
    ticks = 0

    def progress():
        nonlocal ticks
        ticks += 1

    db.set_progress_handler(progress, 100)
    try:
        rows = db.execute(sql, arguments).fetchall()
    finally:
        db.set_progress_handler(None, 0)
    plan = [row[-1] for row in db.execute('EXPLAIN QUERY PLAN ' + sql, arguments)]
    return rows, {'result_rows': len(rows), 'vm_instructions_approx': ticks * 100, 'plan': plan}


def unread_work(event_count, dense):
    with sqlite3.connect(':memory:') as db:
        db.executescript('''
            CREATE TABLE events(id INTEGER PRIMARY KEY,type TEXT,data TEXT);
            CREATE INDEX idx_type ON events(type);
            CREATE TABLE accepted_recipients(name TEXT,event_id INTEGER,PRIMARY KEY(name,event_id)) WITHOUT ROWID;
        ''')
        roster = {'lobo': 0, 'nabe': event_count // 2, 'giru': event_count // 3, 'no-mail': 0}
        relationships = 0
        with db:
            for event in range(1, event_count + 1):
                # Include duplicate historical values and different cursors. The projection deduplicates membership.
                names = ['lobo', 'lobo', 'nabe', 'giru'] if dense else (['lobo', 'lobo', 'nabe'] if event % 1000 == 0 else ['other'])
                db.execute("INSERT INTO events VALUES(?,'message',?)", (event, json.dumps({'delivered_to': names})))
                distinct = set(names)
                db.executemany('INSERT INTO accepted_recipients VALUES(?,?)', ((name, event) for name in distinct))
                relationships += len(distinct)
        legacy = source_sql('src/service/listing_store.rs', "json_extract(data, '$.delivered_to') FROM events WHERE type = 'message'")
        rows, before = measure(db, legacy, (min(roster.values()),))
        expected = Counter({name: 0 for name in roster})
        for event, raw in rows:
            for name in set(json.loads(raw)):
                if name in roster and event > roster[name]:
                    expected[name] += 1
        indexed, after = measure(db, '''
            WITH roster(name,cursor) AS (VALUES (?,?),(?,?),(?,?),(?,?))
            SELECT r.name,(SELECT count(*) FROM accepted_recipients a WHERE a.name=r.name AND a.event_id>r.cursor) FROM roster r
        ''', tuple(value for row in roster.items() for value in row))
        assert dict(indexed) == expected
        return {'events': event_count, 'dense': dense, 'projection_rows': relationships, 'counts': dict(indexed), 'before': before, 'candidate': after}


def thread_work(unrelated):
    with sqlite3.connect(':memory:') as db:
        db.executescript('''
            CREATE TABLE instances(name TEXT PRIMARY KEY);
            CREATE TABLE kv(key TEXT PRIMARY KEY,value TEXT);
            CREATE TABLE members(thread TEXT, member TEXT, created REAL, source_key TEXT, PRIMARY KEY(thread,created,source_key)) WITHOUT ROWID;
        ''')
        db.executemany("INSERT INTO kv VALUES(?,'{}')", ((f'unrelated:{i}',) for i in range(unrelated)))
        active = {'lobo', 'nabe'}
        db.executemany('INSERT INTO instances VALUES(?)', ((name,) for name in active))
        for i, (thread, member, created) in enumerate([('a', 'nabe', 1), ('a', 'lobo', 2), ('a', 'gone', 2), ('b', 'nabe', 3)]):
            key = f'events_sub:member-{i}'
            db.execute('INSERT INTO kv VALUES(?,?)', (key, json.dumps({'thread_name': thread, 'caller': member, 'created': created, 'auto_thread_member': True})))
            db.execute('INSERT INTO members VALUES(?,?,?,?)', (thread, member, created, key))
        # Pre-optimization lookup (retained as a stable experimental control).
        legacy = """SELECT value FROM kv WHERE key LIKE 'events_sub:%' AND json_extract(value, '$.auto_thread_member') = 1
             AND json_extract(value, '$.thread_name') = ? ORDER BY json_extract(value, '$.created') ASC, key ASC"""
        rows, before = measure(db, legacy, ('a',))
        expected = list(dict.fromkeys(value['caller'] for (raw,) in rows if (value := json.loads(raw))['caller'] in active))
        indexed, after = measure(db, '''
            SELECT m.member FROM members m CROSS JOIN instances i ON i.name=m.member
            WHERE m.thread=? ORDER BY m.created,m.source_key
        ''', ('a',))
        assert [name for (name,) in indexed] == expected == ['nabe', 'lobo']
        source = (ROOT / 'src/db/subscription_index.rs').read_text()
        values = {key: re.search(rf'const {key}: &str = "([^"]+)"', source)[1] for key in ('SUBS', 'THREAD_MEMBER', 'THREAD_NAME')}
        ddl = re.search(r'(CREATE INDEX IF NOT EXISTS agentx_thread_member_v1 [^;]+);', source)[1]
        db.execute(ddl.format(**values))
        query = re.search(r'format!\("(SELECT i.name FROM kv [^"]+)"\)', source)[1]
        current, implemented = measure(db, query.format(index='INDEXED BY agentx_thread_member_v1', prefix=values['SUBS'], **values), {'1': 'a'})
        assert [name for (name,) in current] == expected
        assert any('SEARCH' in step and 'agentx_thread_member_v1' in step for step in implemented['plan'])
        return {'unrelated_kv': unrelated, 'before': before, 'candidate': after, 'implemented': implemented}


def stopped_work(event_count):
    with sqlite3.connect(':memory:') as db:
        db.executescript('''
            CREATE TABLE events(id INTEGER PRIMARY KEY,type TEXT,instance TEXT,data TEXT);
            CREATE INDEX idx_type ON events(type);
        ''')
        for event in range(1, event_count + 1):
            data = {'action': 'ready'}
            if event <= 18:
                data = {'action': 'stopped', 'snapshot': {'origin_device_id': 'remote' if event % 3 == 0 else None}}
            db.execute("INSERT INTO events VALUES(?,'life','lobo',?)", (event, json.dumps(data)))
        legacy = source_sql('src/management/store.rs', 'SELECT id,instance FROM events WHERE id < ?1')
        predicate = legacy.split(' AND ', 1)[1].split(' ORDER BY ', 1)[0]
        arguments = {'1': event_count + 1, '2': 100}
        expected, before = measure(db, legacy, arguments)
        db.execute('CREATE INDEX sample_local_stops ON events(id) WHERE ' + predicate)
        _, without_stats = measure(db, legacy, arguments)
        db.execute('ANALYZE events')
        observed, after = measure(db, legacy, arguments)
        assert expected == observed and len(observed) == 12
        return {'life_events': event_count, 'before': before, 'without_statistics': without_stats, 'candidate': after}


if __name__ == '__main__':
    print(json.dumps({
        'sqlite': sqlite3.sqlite_version,
        'vm_counter_granularity': 100,
        'unread': [unread_work(size, dense) for size in (10_000, 100_000) for dense in (False, True)],
        'thread': [thread_work(size) for size in (10_000, 272_000)],
        'stopped': [stopped_work(size) for size in (10_000, 100_000)],
    }, indent=2))
