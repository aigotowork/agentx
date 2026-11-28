---
scope: GitHub Release attachment diagnosis and follow-up
status: diagnosed; repair and publication unverified
last_updated: 2026-11-24
---

# Native release packaging

[v0.7.31](https://github.com/aigotowork/agentx/releases/tag/v0.7.31) has one standalone CLI archive, `agentx-linux-amd64.tar.gz`, plus a Linux Docker image archive, Compose file, license and checksums. The downloaded CLI archive matched `SHA256SUMS`, contained an executable x86-64 Linux ELF, and printed `agentx 0.7.31`. The release has no macOS archives or installer scripts. The [native Release workflow](../../.github/workflows/release.yml) is active, but GitHub reports no runs for it and no tag push run for this release. The only run attached to the tag is a [manual Cloud daemon image run](https://github.com/aigotowork/agentx/actions/runs/44958675805); that workflow publishes to GHCR and does not attach native archives to the GitHub Release. The public evidence does not establish why no tag push run was created.

The tag (`c3e74b9`) and current main have the same [cargo-dist configuration](../../dist-workspace.toml) and generated native Release workflow. A local `dist plan --output-format=json` with the pinned cargo-dist 0.33.0 includes all three required archives: `agentx-aarch64-apple-darwin.tar.gz`, `agentx-x86_64-apple-darwin.tar.gz`, and `agentx-x86_64-unknown-linux-gnu.tar.gz`. The [CI dist check](https://github.com/aigotowork/agentx/actions/runs/45154483754/job/137628315904) validates workflow generation; the current PR mode is `plan`, so it does not validate the archives.

There is a separate blocker for the next release: [the publish job](../../.github/workflows/release.yml#L248) requires the [wheel matrix](../../.github/workflows/build-wheels.yml) to succeed. Wheel jobs compile the same native binary but do not install the locked Web dependencies required by [the build script](../../build.rs#L97). In a clean checkout probe the SDK build failed resolving `packages/agentx-client/node_modules/typescript/bin/tsc`. This explains how wheel builds can block a future native release; it did not cause the missing run for v0.7.31.

For the next release, validate the three native archives in a nonpublishing build and run the packaged `agentx --version` on each matching runner. Ensure the GitHub Release can publish its required native artifacts independently of wheel/PyPI failures. Verify the published assets and their checksums after pushing a new version tag. Treat adding macOS archives to v0.7.31 as a separate backfill from its existing tag commit.
