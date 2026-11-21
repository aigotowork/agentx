---
scope: Reusable launch profile CLI and configuration
status: active
owner: AgentX launch
last_updated: 2026-11-14
related: ["./proposals/launch-profiles.md", "../.controller/launch-profiles/delivery.md"]
---

# 启动 Profile

Profile 为一种工具保存一套具名的原生参数。同一工具可以有多套配置，同一配置可以启动多个 Agent。保存位置是当前 `AGENTX_DIR/config.toml`，省略 `AGENTX_DIR` 时沿用用户默认作用域。

本轮独立构建在 `.bin/agentx-profiles`，下方命令使用产品通用名称 `agentx`。现有 `.bin/agentx`、已安装程序和运行中服务未替换；构建及 Linux 本地检查范围见 [Delivery](../.controller/launch-profiles/delivery.md#release-artifact-and-closure)。

## 保存与启动

```text
agentx profile create codex-daily --tool codex -- --model MODEL_DAILY
agentx profile create codex-deep --tool codex -- --model MODEL_DEEP
agentx profile default codex-daily
agentx profile default codex-daily --tool codex

agentx launch
agentx launch --tool codex
agentx launch --profile codex-deep
agentx launch --profile codex-deep --count 2 --headless
agentx launch --profile codex-deep -- --model MODEL_OTHER
```

第一条 default 设置全局默认，第二条设置 Codex 默认；二者相互独立。默认是省略选择时的指向，其他 profile 不继承它。没有全局默认时，裸 launch 报错；显式指定工具但没有工具默认 profile 时，保留既有工具默认参数路径。

共享角色 `.agent-guides/roles/NAME.md` 可以省略 profile，直接使用这里的默认选择。项目需要特殊配置时，在 `.agentx/roles/NAME.toml` 写 `profile = "NAME"` 即可复用同名 Markdown 指令；启动时显式 `--profile` 优先于角色 profile。发现/检查角色不要求默认 profile，实际角色启动必须冻结一个真实命名 profile。默认 profile 不会自动指定目录消息负责人；该负责人继续由角色 TOML 的 `default = true` 选择。见 [角色用法](roles-workspaces.md)。

显式 profile 优先；同时显式指定工具时必须匹配。显式工具屏蔽 `AGENTX_PROFILE`。没有显式选择时，非空 `AGENTX_PROFILE` 优先于全局默认。不存在的 profile 或失效的选中默认引用会报错。

## 查看与修改

```text
agentx profile list --json
agentx profile show codex-deep --json
agentx launch --profile codex-deep --dry-run --json
agentx profile update codex-deep --description "复杂问题"
agentx profile update codex-deep --replace-args -- --model MODEL_NEW
agentx profile default --unset --tool codex
agentx profile default --unset
agentx profile delete codex-deep
```

show 返回保存的定义，dry-run 返回本次解析结果。dry-run 展示 base、覆盖项、结果和选择原因；本地不启动 daemon、Agent 或工具。原生协议注入及进程事实仍由执行阶段确定。远端 dry-run 使用既有 Relay 通信服务，返回目标设备的解析结果。

update 的参数是整组替换，必须带 `--replace-args`；省略它仅修改明确提供的描述。空 description 清除描述。被任一默认指向的 profile 不能直接删除，应先改变或清除默认。

## 覆盖边界

启动时的参数只影响这一次，不修改 profile。已知单值选项及别名按现有工具规则覆盖；可重复和未知选项保留顺序。原生参数必须放在 `--` 后。

```text
agentx launch --profile codex-deep --replace-args -- --model MODEL_ONLY
agentx launch --profile codex-deep --replace-args
agentx launch --tool codex --no-profile
```

replace 完全替换继承参数；没有额外参数时清空。no-profile 要求显式工具，忽略 profile 和旧工具默认参数。具名 profile 不叠加 `<tool>_args` 或 `AGENTX_<TOOL>_ARGS`，避免两份隐式默认配置混在一起。

目录、数量、初始/系统提示、终端、设备、headless/stdio 和输出方式继续使用启动请求选项。工具自己的原生配置仍由工具读取，因此保存 AgentX 参数不等于冻结整个外部运行环境。

## 历史与远端

启动记录保存实际可复用参数和采用的 profile 定义。修改或删除 profile 不影响已保存的配置；resume/fork 沿用历史参数并接受原有临时覆盖，不重新读取当前默认或 `AGENTX_PROFILE`。原 profile 来源作为历史来源保留，恢复后的有效参数仍以保存的 `launch_args` 为准。

```text
agentx launch --profile codex-deep --device DEVICE --dir /target/project
agentx launch --profile codex-deep --device DEVICE --dir /target/project --dry-run --json
```

远端只在目标设备的 AgentX 作用域解析 profile，本地同名配置不参与。目标缺少该 profile 则失败。新 profile 启动/预览使用明确的 Relay 能力标识，旧设备不会静默忽略选择参数。独立 dry-run 与之后的新启动请求之间，目标配置仍可能改变。

契约与实现边界见[提案](./proposals/launch-profiles.md)，实际检查和构建结果见[Delivery](../.controller/launch-profiles/delivery.md)。
