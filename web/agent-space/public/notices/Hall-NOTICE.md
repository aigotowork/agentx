# Shared hall artwork provenance

Pure bitmap/palette/PNG helpers and walking paths are adapted from Agent Lobby,
https://github.com/linhmh8/agent-lobby.git, fixed revision
e23536c1dd5493d422579d833a77658403b1b4a9 (v0.1.6).

Upstream declares MIT in package.json and README; its fixed snapshot and original
notices remain in repos/lobby and repos/lobby-source.json. No standalone LICENSE
or additional copyright notice existed in the snapshot; none is invented here.
The sole adaptation removes the palette's host Theme type dependency in favor of
a structural color input. These modules contain no React, DOM, RN or host RPC.
The shared hall controller and product projection are bots-owned.
