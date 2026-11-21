#!/usr/bin/env bash
# Package a native Linux release executable built on Ubuntu 24.04 or compatible.
set -euo pipefail
cd "$(dirname "$0")/.."
binary=${1:-target/release/agentx}
image=${2:-agentx-cloud:local}
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
install -m 755 "$binary" "$stage/agentx"
cp deploy/Dockerfile LICENSE "$stage/"
labels=()
if [[ -n ${GITHUB_REPOSITORY:-} && -n ${GITHUB_SHA:-} ]]; then
    labels+=(--label "org.opencontainers.image.source=https://github.com/$GITHUB_REPOSITORY"
             --label "org.opencontainers.image.revision=$GITHUB_SHA")
fi
docker build "${labels[@]}" --tag "$image" "$stage"
