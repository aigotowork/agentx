#!/usr/bin/env bash
set -euo pipefail

repo_root="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ "$(uname -o 2>/dev/null || true)" == "Android" ]]; then
  # TypeScript 7 ships the compiler as a per-platform native (Go) binary and
  # publishes no `@typescript/typescript-android-*` package, so `tsc` cannot
  # resolve an executable here at all. Substituting the linux-arm64 build does
  # not help either: Android's seccomp filter kills it with SIGSYS on startup.
  # The typecheck gate therefore only runs on the ubuntu CI job (see the
  # `typecheck` job in .github/workflows/ci.yml); skipping loudly here keeps a
  # local `just ci` honest about what it did and did not verify.
  if [[ "${AGENTX_TYPECHECK_FORCE:-}" != "1" ]]; then
    echo "typecheck: SKIPPED on Android (TypeScript 7 has no android native build)"
    echo "typecheck: plugin types are gated by the ubuntu CI job; set AGENTX_TYPECHECK_FORCE=1 to attempt it anyway"
    exit 0
  fi

  # Stage the plugin sources under a dedicated child dir so the rm -rf below
  # can never target a caller-supplied path directly (e.g. AGENTX_TYPECHECK_ROOT
  # pointed at the repo would otherwise wipe the whole src/ tree).
  project_root="${AGENTX_TYPECHECK_ROOT:-$HOME/.agentx/.cache}/agentx-typecheck-stage"
  if [[ "$project_root" == "$repo_root" ]]; then
    echo "typecheck: refusing to stage into the repo root ($repo_root)" >&2
    exit 1
  fi

  rm -rf "$project_root/src"
  mkdir -p "$project_root/src/integrations/plugins" "$project_root/tests"
  cp "$repo_root/package.json" "$repo_root/package-lock.json" "$repo_root/tsconfig.json" \
    "$project_root/"
  cp -R "$repo_root/src/integrations/plugins/omp" "$repo_root/src/integrations/plugins/opencode" \
    "$repo_root/src/integrations/plugins/pi" "$project_root/src/integrations/plugins/"
  cp "$repo_root/tests/plugin_delivery.test.ts" "$repo_root/tests/plugin_types.ts" \
    "$repo_root/tests/dependency_compatibility.test.mjs" "$project_root/tests/"
else
  project_root="$repo_root"
fi

cd "$project_root"
# `npm ci`, never `npm install`: it installs exactly the lockfile and never
# writes it. `npm install` rewrote package-lock.json in the format of
# whichever npm ran it (npm versions disagree on recording `libc` fields),
# leaving the checkout dirty after every local check.
if [[ "${CI:-}" == "true" ]]; then
  npm ci --ignore-scripts
elif [[ ! node_modules/.package-lock.json -nt package-lock.json \
  || package.json -nt node_modules/.package-lock.json ]]; then
  # npm ci wipes node_modules (over a minute here), so locally it runs only
  # when the manifests changed since the last install, which npm records in
  # node_modules/.package-lock.json.
  #
  # CI enforces the pinned Node 22 runtime. Local typechecking can also run on a
  # newer Node even when the user's global npm config enables engine-strict.
  npm ci --ignore-scripts --prefer-offline --engine-strict=false
fi
npm run typecheck
npm run test:dependencies
npm run test:plugins
# The Hermes plugin is Python (stdlib only); its fake-Hermes regressions.
python3 -I "$repo_root/tests/hermes_plugin.py"
