#!/usr/bin/env bash
# Freeze one synthetic Space candidate and start its own loopback preview.
set -euo pipefail
repo="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo"
revision="${1:-}"
port="${2:-}"
[[ "$revision" =~ ^r[1-9][0-9]*$ && "$port" =~ ^190[0-9][0-9]$ ]] || { echo "usage: bash candidate.sh rN 190NN" >&2; exit 2; }
output="$repo/.artifacts/space-conversation-workbench/candidate-$revision"
mkdir "$output"
cp -p web/agent-space/tests/workspace.fixture.mjs "$output/workspace.fixture.mjs"
node web/agent-space/build.mjs "$output/space" > "$output/build.log" 2>&1
systemd-run --user --collect --unit "agentx-space-workbench-isolated-$revision" --working-directory "$repo" "$(command -v node)" "$output/workspace.fixture.mjs" "$output/space" "$port" 127.0.0.1 workbench
curl --noproxy '*' --fail --silent --max-time 3 --retry 5 --retry-connrefused "http://127.0.0.1:$port/api/v1/management"
