# Changelog

## [0.7.33]

- Distribute the native CLI and its updates through GitHub Releases. Legacy package installs now receive migration guidance instead of launching a package manager.
- Verify a downloaded update before replacing the running CLI, and check release version consistency in CI.

## [0.7.32]

- Build and verify standalone AgentX archives for Apple Silicon, Intel Macs, Linux AMD64 and Windows AMD64 in GitHub Actions.
- Publish the native archives and installers to GitHub Releases independently of the cloud daemon image.
