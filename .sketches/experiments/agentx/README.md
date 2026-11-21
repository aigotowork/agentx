# agentx 交互实验

这是按用户要求保留的真实实验。实际 agent 是 `nova`，工作 Session 是
`s_e7cad398de31`。查看 [任务进度](task.md)、[问候脚本](greeting.py)、
[目录角色](.agent-guides/roles/demo-writer.md) 和
[Session 原始文件](.agentx/sessions/s_e7cad398de31/session.yaml)。

## 启动新 agent

从项目根目录运行下面的命令，会新建一份工作 Session，再由 agentx 启动新 agent。
新 agent 的名字由 agentx 自动生成，不需要预先有一个 agentx agent，也不用指定 `vali`。
下面只是展示启动方式；已有的 `nova` 可以直接查看，不必重复启动。

```sh
.bin/agentx launch \
  --dir experiments/agentx \
  --role demo-writer \
  --skill agentx \
  --task experiments/agentx/task.md \
  --env DEMO_PREFIX=你好
```

Role 默认绑定 `minimalist`。`--skill agentx` 和 `--env DEMO_PREFIX=你好`
用于演示额外绑定和环境注入，并不是所有启动都需要的参数。

## 运行中的 agent 自己使用

agentx 自动注入 `AGENTX_SESSION_DIR`，agentx 提供自身调用身份。
agent 不必抄写 Session ID、目录或 agentx 名称，改变 CWD 后也一样：

```sh
agentx mycontext
agentx status
# 任务进度：直接编辑 task.md 的 YAML frontmatter
```

其中 `mycontext` 组合角色、技能、原任务和当前 agentx 观察；`status`
读取任务状态、Role/Skill 来源可用性，并附带 agentx 运行观察。任务由 agent 直接维护原文件。

## 你从终端查看实验

从项目根目录探索可用角色、技能和这个目录已有的 agent：

```sh
.bin/agentx discover --dir experiments/agentx
```

外部终端没有自动绑定某个工作 Session，因此查看指定对象时需要给它的 ID。
进入所属目录后，不必再重复 `--dir`：

```sh
cd experiments/agentx
../../.bin/agentx mycontext --session s_e7cad398de31
../../.bin/agentx status --session s_e7cad398de31
```

`--session` / Session 参数选择目标；`--dir` 只是所属目录，默认当前目录。
多个 Session 存在时不猜测最新一个。agentx 调用身份使用原生机制处理，
agentx 不再提供容易与新 agent 名称混淆的调用者参数。

任务第一阶段已生成问候脚本，验证环境与 `/tmp` 下的上下文恢复，然后等待你
提出后续修改。`task.md` 的 `blocked` 表示等待后续要求；agentx 的 `listening`
表示 agent 在线、空闲等待。

在项目根目录运行产物：

```sh
python3 experiments/agentx/greeting.py 小明
python3 experiments/agentx/greeting.py 小明 --prefix 早上好
```

可以在当前对话告诉助手下一步要求，例如“让 nova 增加 `--repeat N` 参数”。
也可以使用 agentx 原生通信。后续执行仍维护同一个 `task.md`，不另外生成报告。
停止 agent 使用 agentx 的原生管理方式；恢复这个工作 Session 可以运行：

```sh
.bin/agentx resume s_e7cad398de31 --dir experiments/agentx
```

恢复沿用保存的底层身份引用，不会重新创建一份 agentx 工作 Session。

## 当前工具边界

`discover` 用于发现；`launch` / `resume` 用于带绑定的启动和继续；
`status` 是资源摘要，以原任务状态为主；`mycontext` 是完整上下文恢复。
工具不提供任务操作。agent 直接编辑 `task.md` 的状态、进度、阻塞、下一步与结果，
agentx 再只读消费它们，不改变任务负责人，也不执行任务状态流转。
本实验的角色来源和当前 Session 中的角色说明已同步到这一约定。
