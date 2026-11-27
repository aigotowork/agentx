# Changelog

## [0.7.36]

- Use `agentx docs` for the CLI reference, configuration guide and workflow authoring documentation. Use `agentx events sub --help` for one-time reminder examples and outcome checks.
- Includes the Codex recovery and Unix daemon socket fixes from 0.7.35.

## [0.7.35]

- Recover a live Codex Agent whose failed turn ended without a Stop hook. Recovery checks the latest completed turn in the same session before offering or submitting a continuation.
- Running Unix daemons restore missing native and observation socket paths after temporary-directory cleanup. Startup waits for the scope owner, and restart can reconnect and authenticate it during repair.

## [0.7.34]

- Native AgentX commands now send operations directly. Unsupported features fail individually, and a lost send response can be checked with `agentx send --receipt REQUEST_ID`.
- To upgrade a daemon from native protocol 3 or 4, run the new binary's `agentx daemon restart --scope PATH` once. Reopen Agent sessions still using an older host after the daemon is ready.
- Space and the Domain SDK keep available features usable when a service lacks Workspace members or message history. Stale Scope data no longer reuses old cursors or recipients.
- Cloud Space uses Namespace v2 and checks the selected generation during each Agent, Host, and Workspace page read. Upgrade the cloud service before switching dependent clients.
- Codex PTY delivery now waits for readiness when the terminal is starting or shows a menu, approval, or existing input. It rechecks the screen and local input before inserting, and does not type a second wake while the original wake's acknowledgement is unresolved.
- Retire keyless Relay join tokens and old configuration read fallbacks.

## [0.7.33]

- Set a one-shot local reminder with `agentx events sub --in 10m --message 'Check the GitHub build status'`. Use `sub list --history` and `sub show` to inspect outcomes, or `events unsub` to cancel a pending reminder.
- Reminders survive daemon restarts and deliver to the exact Agent instance. A stopped or replaced target is recorded as a failed reminder.
- AgentX native IPC now uses protocol 4. Upgrade the daemon, CLI and running Agent hosts together; protocol 3 and 4 cannot connect.

## [0.7.32]

- Build and verify standalone AgentX archives for Apple Silicon, Intel Macs, Linux AMD64 and Windows AMD64 in GitHub Actions.
- Publish the native archives and installers to GitHub Releases independently of the cloud daemon image.
