#!/bin/sh
set -eu

NEW_URL="https://github.com/aigotowork/agentx/releases/latest/download/agentx-installer.sh"

cat <<EOF
install.sh has moved and this compatibility shim will be removed in a future release.
Please update your command to:

  curl -fsSL $NEW_URL | sh

Running the new installer now...
EOF

SHIM_TMP=$(mktemp)
curl -fsSL "$NEW_URL" -o "$SHIM_TMP"
sh "$SHIM_TMP"
rm -f "$SHIM_TMP"
