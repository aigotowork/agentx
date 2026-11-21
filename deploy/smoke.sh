#!/usr/bin/env bash
# Exercise the same Compose service and persistent volume used by operators.
set -euo pipefail
cd "$(dirname "$0")"
export AGENTX_IMAGE=${1:-agentx-cloud:local}
export AGENTX_PORT=${AGENTX_SMOKE_PORT:-$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')}
export AGENTX_PUBLIC_URL="http://127.0.0.1:$AGENTX_PORT"
AGENTX_ADMIN_TOKEN=$(openssl rand -hex 32)
export AGENTX_ADMIN_TOKEN
compose=(docker compose -p "agentx-smoke-$$" -f compose.yaml)
cleanup() {
    if [[ $? != 0 ]]; then "${compose[@]}" logs --tail 30 cloud >&2; fi
    "${compose[@]}" down --volumes >/dev/null
}
trap cleanup EXIT
ready() {
    curl -fsS --retry 15 --retry-delay 1 --retry-connrefused "$AGENTX_PUBLIC_URL/deployment" |
        grep -i '<!doctype html' >/dev/null
}
admin() {
    curl --fail-with-body -sS -H "Authorization: Bearer $AGENTX_ADMIN_TOKEN" "$@"
}
for api in admin scope; do
    diff <(jq -S . "../packages/agentx-client/cloud-$api.openapi.json") \
        <(docker run --rm --network none "$AGENTX_IMAGE" cloud-openapi "$api" | jq -S .)
done
scope="$AGENTX_PUBLIC_URL/api/v1/admin/scopes/smoke"
"${compose[@]}" up -d --pull never
ready
test "$(curl -sS -o /dev/null -w '%{http_code}' "$scope")" = 401
receipt=$(admin -X PUT -H 'Content-Type: application/json' \
    -d "{\"request_id\":\"container-smoke\",\"token\":\"$(openssl rand -hex 32)\"}" "$scope")
generation=$(jq -er '.generation | select(length > 0)' <<<"$receipt")
"${compose[@]}" up -d --force-recreate --pull never
ready
admin "$scope" | jq -e --arg gen "$generation" '.available and .generation == $gen' >/dev/null
