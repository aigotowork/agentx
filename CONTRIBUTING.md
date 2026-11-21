# Contributing

Issues and pull requests are welcome. Thanks for contributing!

## Development

Prerequisites: Rust 1.89+, Node.js and TypeScript for the embedded browser console. See pinned [Rust toolchain](rust-toolchain.toml) and [standalone packaging](#standalone-binary-packaging).

```bash
git clone https://github.com/aigotowork/agentx.git
cd agentx
npm ci --prefix packages/agentx-client
npm ci
npm ci --prefix web/inspector
cargo build
cargo test
```

### Using a local build

**Symlink** — simple, dev build is global.

```bash
ln -sf "$(pwd)/target/debug/agentx" ~/.cargo/bin/agentx
```

**dev_root** — works regardless of how agentx was installed (brew, pip, etc.); picks the newer of debug/release automatically:

```bash
agentx config dev_root "$(pwd)"
agentx config dev_root --unset  # revert
agentx status                   # run local build
```

For concurrent worktrees, scope each to its own DB:

```bash
AGENTX_DIR=$PWD/.agentx AGENTX_DEV_ROOT=$PWD agentx launch --tool claude
```

Source ownership and module organization: [src/README.md](src/README.md).

### Files for a public checkout

Commit source, lockfiles, test fixtures, published SDK contracts and required runtime assets. Keep `.agent-guides/` and the `.harness/manifest.json` that records its files together. Harness upgrades still come from the upstream harness project.

Keep local environment and MCP configuration, runtime state, harness backups, test output, screenshots and build output out of Git. Use `.artifacts/` or a temporary directory for local captures. Source and durable design or delivery notes can remain in the repository; links to historical local evidence may only resolve in the workspace that produced it.

For direnv, copy [`.envrc.example`](.envrc.example) to `.envrc` and run `direnv allow`. To set local Provider options, copy [`.env.local.example`](.env.local.example) to `.env.local`, fill in the values you need, and run `chmod 600 .env.local`. Git ignores both local files.

### Standalone binary packaging

`cargo build` (including `just package`) compiles the diagnostic console (`web/observability`), collaboration workbench (`web/workbench`) and Agent management console (`web/management`), then embeds their finite HTML/CSS/JavaScript bundles. `just package` writes `.bin/agentx`; packaged users need only that executable. The diagnostic and collaboration entries consume separate tables and preserve their distinct runtime scopes. External `--assets` bundles remain an optional development input.

For checkout and extracted Cargo source-archive builds, first install the exact root npm lock with Node `22.23.3` and npm `10.9.9`. Archives include all browser source trees, package.json/package-lock.json and builders, but exclude node_modules and generated bundles. The React/Vite workbench requires the local locked dependencies, so a globally installed tsc alone is insufficient. Cargo does not install npm packages or fetch compilers. A source archive intended for cargo install must have its root npm dependencies prepared before invoking Cargo; prebuilt binary users need no Node.js.

The public browser SDK is now a local package dependency: prepare `npm ci --prefix packages/agentx-client`, `npm ci` and `npm ci --prefix web/inspector` before Cargo (see [Web SDK setup](packages/agentx-client/README.md#web-consumers)). Cargo compiles the public SDK once before building the five apps; standalone Web checks first need `npm run build --prefix packages/agentx-client`. Observation uses the same root Vite bundler so npm imports are fully bundled. The build watches SDK source/scripts/config/manifests, all browser trees, npm manifests, relevant installed compiler/bundler/React inputs and existing native build inputs. Fresh bundles are emitted beneath Cargo OUT_DIR, with separate sorted `observability_assets.rs`, `workbench_assets.rs` and `management_assets.rs` tables. A frontend type/build failure aborts Cargo instead of embedding stale output. The workbench builder uses the exactly matched official Rolldown WASI binding before dynamically importing Vite, avoiding the observed BMI2 native-initializer crash on acceptance's Linux host. The scoped Vite/Rolldown override and WASI package must be updated together; missing or mismatched binding fails the build rather than falling back to the native module. Node's WASI experimental warning, the current Vite config's future-major warning and upstream concurrent-instance memory-growth limits remain documented in the [portable builder record](.docs/collaboration-workbench/frontend.md#portable-bundler-correction--web-workbench-r12); this is not a universal platform/concurrency guarantee. The workbench builder accepts one new absolute disposable output directory and emits `{files: [{url, file, content_type}]}` with flat hashed JS/CSS and `/` as the index URL. Its assets use same-origin external scripts/styles without a CDN or inline bootstrap.

Use `just workbench-typecheck` for the scoped TypeScript check and `just workbench-build /absolute/new/output` for a disposable bundle after dependencies are installed. Independent acceptance owns actual execution during the active Delivery. Packaging evidence must include binary-only startup from an unrelated directory and source-input invalidation; see the [workbench frontend entry](.docs/collaboration-workbench/frontend.md) and [acceptance Task](.controller/collaboration-workbench/tasks/acceptance.md).

### Open the collaboration workbench

With an actual built or packaged executable, choose existing absolute scope/project directories and an available nondefault port:

```bash
/absolute/path/to/agentx workbench --scope /absolute/path/to/scope --project /absolute/path/to/project --name wb-operator --collaboration --host 127.0.0.1 --port 18947
```

Open the printed URL. Replace `127.0.0.1` with this machine's numeric LAN interface address for direct LAN access. Reaching clients can use the finite credential-free operator commands enabled by `--collaboration`; omit that flag for read-only access to an existing daemon. Scope/project/mode stay fixed for that listener. `--name` is accepted for CLI compatibility and does not identify a browser as an Agent. Ctrl-C closes the adapter; Provider/daemon lifecycle is separate. Original passive diagnostics use `agentx observe-web`.

The [startup and use guide](.docs/collaboration-workbench/frontend.md#startup-and-everyday-use) covers actual role selection, separate save/dispatch, exact questions, immutable results, independent checks, repair/acceptance and unknown-response recovery. The [support record](.docs/collaboration-workbench/frontend.md#independently-exercised-support) links the observed Linux two-Agent outcome and final r8 binary's embedded workbench/diagnostic and narrow/touch checks. The [final handoff](.controller/collaboration-workbench/evidence/acceptance.md#final-artifact-evidence-and-cleanup-handoff) owns exact artifact/source identities, same-byte repository `.bin/agentx` promotion and completed disposable-runtime cleanup. Installed/PATH/default programs and the existing LAN demonstration remain unchanged; formal Delivery acceptance remains controller/user-owned.

### 领域模型设计工作台

[Flow Atlas 工作台](tools/flow-atlas/workbench/README.md)是源码优先的独立开发工具：领域模型、原生Schema／方法引用、规则、流程参数绑定和场景由TS定义；浏览器消费导出JSON及其检查点。它不需要启动AgentX或构建Rust程序。实现已通过[独立本地验收](.controller/domain-daemon/flow-atlas-workbench-implementation.md#accepted-local-outcome)，可打开[局域网演示](http://192.168.2.178:18959/)或[离线页面](tools/flow-atlas/examples/domain-workbench.html)。

使用Node22.23.3及已准备的仓库锁定依赖，选择一个尚不存在的JSON输出文件：

```bash
node --experimental-strip-types tools/flow-atlas/domain/examples/export.ts order-stock /tmp/my-domain.json
python3 tools/flow-atlas/workbench/build.py /tmp/my-domain.json --output /tmp/my-domain.html
```

在浏览器直接打开HTML。把`order-stock`换成`technical`可导出不需要实体／聚合的技术领域示例。导出与查看不执行业务方法；实际原生调用必须由消费者显式发起。首次接入自己的TS项目，从[领域公共契约与示例](tools/flow-atlas/domain/README.md)开始；新增规则按[诊断维护说明](tools/flow-atlas/domain/lint.md)补有意义的反例，保持Schema与业务字段的唯一来源。

源文件修改后重新导出并构建；JSON／HTML是可重建投影，布局JSON只保存画面位置。相邻HTML收据记录精确输入／输出。源码类型检查入口是`node_modules/.bin/tsc -p tools/flow-atlas/domain/tsconfig.json`；独立行为／原生／浏览器案例为`tests/flow_atlas_domain.mts`、`tests/flow_atlas_domain_native.mts`和`tests/flow_atlas_domain_browser.mjs`，其参数与实际覆盖由[验收Task](.controller/domain-daemon/tasks/flow-atlas-domain-acceptance.md#accepted-final-derivatives-and-publication)拥有。该Delivery已完成；后续只运行改动涉及的有意义案例，不重启历史团队或重复无关检查。浏览器没有业务执行或语义源码回写能力，oRPC自动导入、通用循环／并发及任意源码流程推断尚未实现。

### Plugin dependencies

Use Node `22.23.3` and npm `10.9.9` for the pinned plugin check environment. Run `just typecheck` to install the lockfile and check the real upstream SDK types, dependency compatibility, plugin delivery and Hermes fixtures. The local recipe also accepts newer Node versions. A direct `npm run typecheck` assumes dependencies are already installed.

`just audit` (or `npm run audit:dependencies`) audits the entire lockfile, including development dependencies, and fails on any reported vulnerability. It does not require `node_modules`; the existing CI plugin job also runs it. This graph supports development checks and is separate from the Rust executable and the user's installed provider tools.

Keep upstream SDK profiles exact in [package.json](package.json). Prefer an upstream compatible fix when available. Some SDKs still pin vulnerable libraries, so the selected security overrides target affected version ranges and preserve each consumer's dependency major where possible. Cross-major matcher/parser replacements have consumer compatibility cases in [dependency_compatibility.test.mjs](tests/dependency_compatibility.test.mjs); Puppeteer's same-major update supplies its own newer BiDi/Zod graph. Retire overrides when upstream no longer needs them. Do not replace real SDK declarations, suppress advisories, omit development dependencies or accept an unchecked SDK downgrade to obtain a clean audit. The [maintenance record](.controller/plugin-dependency-maintenance/task.md) retains the original findings, selection basis and execution receipts.

## CI

Run the full CI gate locally:

```bash
just ci
```

Runs: release-workflow check, dependency audit, fmt, clippy, typecheck, msrv, unit tests, integration tests, and live relay test. See [justfile](justfile) for details.

`just ci` integration tests run genuine Codex/Claude binaries against local mock providers. No model/API usage.

## Debugging

```bash
agentx status --logs
agentx term debug --help # access pty log file
ls -a ~/.agentx/ ~/.agentx/.tmp/ # config, env, db, integrations, logs, launch scripts
```

## Pull requests

- Live test with the real tool
- Prefer preserving the underlying agent behavior
- Prefer shared behavior across tools
- Support latest agent version
