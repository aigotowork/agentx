# Changelog

## [0.7.33]

- Set a one-shot local reminder with `agentx events sub --in 10m --message 'Check the GitHub build status'`. Use `sub list --history` and `sub show` to inspect outcomes, or `events unsub` to cancel a pending reminder.
- Reminders survive daemon restarts and deliver to the exact Agent instance. A stopped or replaced target is recorded as a failed reminder.
- AgentX native IPC now uses protocol 4. Upgrade the daemon, CLI and running Agent hosts together; protocol 3 and 4 cannot connect.

## [0.7.32]

- Build and verify standalone AgentX archives for Apple Silicon, Intel Macs, Linux AMD64 and Windows AMD64 in GitHub Actions.
- Publish the native archives and installers to GitHub Releases independently of the cloud daemon image.
