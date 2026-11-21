# Environment loading acceptance fixtures

This directory contains consumer-owned, disposable fixtures for the environment-file Delivery. The scenario contract and expected observations live in [README.md](README.md); `prepare.py` creates a fresh run root and `codex_stub.py` is the finite local native recipient.

The fixtures never target the repository `.agentx`, the running workbench, the protected demo, or installed programs. A run must pass an explicit temporary root and retain its manifest, process identities, cleanup receipt, and redacted observations under the acceptance evidence root.
