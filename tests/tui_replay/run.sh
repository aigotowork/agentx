#!/usr/bin/env bash
set -euo pipefail

scenario=${1:-all}
test_filter=(tui::replay::)
info_scenarios=(info_links info_identity info_context info_resize)
scenarios=(mixed geometry modal inline quick_launch quick_launch_vertical "${info_scenarios[@]}")
case "$scenario" in
    all) ;;
    quick_launch|quick_launch_vertical)
        test_filter=(tui::replay::quick_launch -- --exact)
        scenarios=(quick_launch quick_launch_vertical)
        ;;
    info)
        test_filter=(tui::replay::info_)
        scenarios=("${info_scenarios[@]}")
        ;;
    *)
        found=false
        for name in "${scenarios[@]}"; do
            if [[ $name == "$scenario" ]]; then found=true; break; fi
        done
        if [[ $found == false ]]; then
            printf 'Unknown replay scenario: %s (choose %s, info, or all)\n' "$scenario" "${scenarios[*]}" >&2
            exit 2
        fi
        test_filter=("tui::replay::$scenario" -- --exact)
        scenarios=("$scenario")
        ;;
esac

export AGENTX_TUI_REPLAY_OUTPUT="$PWD/target/tui-replay"
export TZ=UTC
cargo test --locked --bin agentx "${test_filter[@]}"
node tests/tui_replay/export.mjs "$AGENTX_TUI_REPLAY_OUTPUT" "${scenarios[@]}"
