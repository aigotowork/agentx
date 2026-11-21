# Browser Explorer

把 [browser-explorer.toml](browser-explorer.toml) 放到目标项目的 `.agentx/roles/`。将 [profile.toml](profile.toml) 中的 `codex-browser-explorer` 条目合并进所选 `$AGENTX_DIR/config.toml` 的 `launch.profiles`。如果现有 `profiles` 是内联表，把该条目加到表里，或将现有条目一起改用标准 TOML 表；不要覆盖整份配置。

从目标项目预览并启动：

```sh
agentx launch --role browser-explorer --dry-run
agentx launch --role browser-explorer --headless --go --agentx-prompt "探索所给页面，记录当前结构和状态，并生成 Playwright TypeScript 脚本"
```

该 Profile 用原生参数把 Playwright MCP 配置传给 Codex，使用现有 `CODEX_HOME`。迁移时只需带走 Role 和 AgentX Profile，无需安装同名 Codex profile。运行仍需要 Codex、Node.js/npm 和可用的 Playwright Chrome 环境；初次执行可能下载固定版本的 MCP 包。其他 AgentX Profile 仍可在 `args` 中使用 Codex 原生 `--profile NAME`。
