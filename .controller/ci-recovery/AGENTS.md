# GitHub CI recovery

The [workflow](../../.github/workflows/ci.yml) owns the release-independent test matrix. Check each failing job's first compiler or assertion diagnostic. For terminal and relay reproduction use the pinned provider versions and the [test harness](../../tests/support/mod.rs). Verify Linux and Windows results individually. A green container publish checks a different surface. Initial failing run: [44959821752](https://github.com/aigotowork/agentx/actions/runs/44959821752).
