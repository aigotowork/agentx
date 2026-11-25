#!/usr/bin/env bash
# Exercise the same Compose service and persistent volume used by operators.
set -euo pipefail
cd "$(dirname "$0")"
export AGENTX_IMAGE=${1:-agentx-cloud:local}
export AGENTX_PORT=${AGENTX_SMOKE_PORT:-$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')}
export AGENTX_PUBLIC_URL="http://127.0.0.1:$AGENTX_PORT"
AGENTX_ADMIN_TOKEN=$(docker run --rm --network none "$AGENTX_IMAGE" cloud-token)
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
for api in admin namespace; do
    diff <(jq -S . "../packages/agentx-client/cloud-$api.openapi.json") \
        <(docker run --rm --network none "$AGENTX_IMAGE" cloud-openapi "$api" | jq -S .)
done
namespace="$AGENTX_PUBLIC_URL/api/v1/admin/namespaces/smoke"
"${compose[@]}" up -d --pull never
ready
test "$(curl -sS -o /dev/null -w '%{http_code}' "$namespace")" = 401
namespace_token=$(docker run --rm --network none "$AGENTX_IMAGE" cloud-token)
receipt=$(admin -X PUT -H 'Content-Type: application/json' \
    -d "{\"request_id\":\"container-smoke\",\"token\":\"$namespace_token\"}" "$namespace")
test "$(jq -er '.namespace_id' <<<"$receipt")" = smoke
namespace_url=$(jq -er '.namespace_url' <<<"$receipt")
test "$namespace_url" = "$AGENTX_PUBLIC_URL/namespaces/smoke"
curl -fsS -H "Authorization: Bearer $namespace_token" "$namespace_url/api/v1/workspaces" |
    jq -e '.items == [] and .next == null' >/dev/null
test "$(curl -sS -o /dev/null -w '%{http_code}' "$AGENTX_PUBLIC_URL/scopes/smoke/space/")" = 404
generation=$(jq -er '.generation | select(length > 0)' <<<"$receipt")
"${compose[@]}" up -d --force-recreate --pull never
ready
admin "$namespace" | jq -e --arg gen "$generation" '.available and .generation == $gen' >/dev/null
