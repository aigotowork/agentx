"""Refresh the offline public command snapshot from an explicitly selected binary."""
import json
from pathlib import Path
import re
import subprocess
import sys

root = Path(__file__).resolve().parent
binary = Path(sys.argv[1]).resolve()
help_text = subprocess.run([str(binary), '--help-all'], check=True, capture_output=True, text=True).stdout
parts = re.split(r'^=== agentx(.*?) ===\n', help_text, flags=re.M)
pages = {parts[i].strip(): parts[i + 1].strip() for i in range(1, len(parts), 2)}
if not pages or 'work report' not in pages or 'observe' not in pages:
    raise SystemExit('Missing expected command help pages')
(root / 'help-source.txt').write_text(help_text)
target = root / 'catalog.js'
existing = target.read_text() if target.exists() else ''
marker = '// GENERATED HELP SNAPSHOT\n'
existing = existing.split(marker)[0]
target.write_text(existing + marker + 'const HELP = ' + json.dumps(pages, ensure_ascii=False, indent=2) + ';\n')
print(f'Refreshed {len(pages)} public help pages from {binary}')
