---
scope: GitHub Release attachment diagnosis and follow-up
status: implementation under validation
last_updated: 2026-11-24
---

# Native release packaging

[v0.7.31](https://github.com/aigotowork/agentx/releases/tag/v0.7.31) has one standalone CLI archive, `agentx-linux-amd64.tar.gz`, plus a Linux Docker image archive, Compose file, license and checksums. The downloaded CLI archive matched `SHA256SUMS`, contained an executable x86-64 Linux ELF, and printed `agentx 0.7.31`. The release has no macOS archives or installer scripts. The [native Release workflow](../../.github/workflows/release.yml) is active, but GitHub reports no runs for it and no tag push run for this release. The only run attached to the tag is a [manual Cloud daemon image run](https://github.com/aigotowork/agentx/actions/runs/44958675805); that workflow publishes to GHCR and does not attach native archives to the GitHub Release. The public evidence does not establish why no tag push run was created.

The [v0.7.31 cargo-dist configuration](https://github.com/aigotowork/agentx/blob/v0.7.31/dist-workspace.toml) already listed both Macs and Linux AMD64. The [historical CI dist check](https://github.com/aigotowork/agentx/actions/runs/45154483754/job/137628315904) validated workflow generation; the old PR mode was `plan`, so it did not validate the archives.

The [v0.7.31 publish job](https://github.com/aigotowork/agentx/blob/v0.7.31/.github/workflows/release.yml#L248) required the wheel matrix to succeed, but wheel jobs did not install the locked Web dependencies required by the build script. In a clean checkout probe the SDK build failed resolving `packages/agentx-client/node_modules/typescript/bin/tsc`. This did not cause the missing run for v0.7.31. The old Homebrew tap is unavailable and the public `agentx` name on PyPI belongs to another project.

The native Release workflow now plans Linux AMD64, both Macs, and Windows AMD64 in [cargo-dist](../../dist-workspace.toml). PRs build archives without publishing, and the [archive check](../../.github/workflows/verify-native-artifacts.yml) validates the actual bytes on each runner before tag publication. Homebrew and PyPI are disconnected from the native release; GHCR stays independent. The target is v0.7.32 after the PR and tag runs pass. Follow the [operator runbook](../../CONTRIBUTING.md#github-native-release); leave v0.7.31 intact.
