# 用 Docker 部署 AgentX 云端管理端

云端容器负责 scope、消息和管理操作。各工作主机上的原生 daemon 主动通过 HTTPS 接入，在自己的工作目录运行 Provider。下面的 Compose 配置适用于 Linux Docker Engine，GitHub 镜像面向 linux/amd64。

## 使用已发布镜像

将本目录的 `compose.yaml` 放到部署目录，在同一目录执行以下命令。将镜像地址替换为 GitHub Packages 中实际发布的版本；私有包需先登录 GHCR。首次初始化一次管理凭据，后续启动沿用同一份文件。

```sh
(umask 077; set -C; echo "AGENTX_ADMIN_TOKEN=$(openssl rand -hex 32)" > .env)
export AGENTX_IMAGE=ghcr.io/OWNER/REPO-cloud:vX.Y.Z
docker compose pull
docker compose up -d
docker compose logs --tail=20 cloud
```

浏览器访问 `http://127.0.0.1:18755/deployment`，使用 `.env` 中的管理 token 创建 scope。容器以 UID 10001 运行，Compose 命名卷保存部署目录。默认 Compose 项目名来自部署目录名，升级时沿用该目录和项目名；先停止服务并备份整个卷，再切换镜像版本并运行 `docker compose up -d`。`docker compose down` 会保留数据，`down -v` 会删除卷。

## 打开云端 Space

在包含云端 Space 的镜像中，从部署控制台选中 scope，点击「打开 Space」，也可以直接访问 `/scopes/{id}/space/`。使用 Admin token 可选择主机和角色、启动 Agent、停止选定实例并查询回执。启动已排队时，等回执显示执行成功，再刷新清单。使用该 scope 的 token 可查看主机、Agent 和选定实例的云端消息。切换页面后需重新输入对应 token。详见 [Space 使用说明](../web/agent-space/README.md#云端-space)。

## 让其他机器接入

在 `.env` 中设置 `AGENTX_PUBLIC_URL=https://agents.example.com`，然后重新执行 `docker compose up -d`。在宿主机 HTTPS 反向代理上将整个域名转发至 `http://127.0.0.1:18755`，保留请求路径和 Authorization 请求头，并允许超过 20 秒的长轮询。例如，使用宿主机上的 Caddy：

```caddyfile
agents.example.com {
    reverse_proxy 127.0.0.1:18755
}
```

此配置使用宿主网络，让 daemon 继续只监听回环地址。如需更换端口，在 `.env` 设置 `AGENTX_PORT` 并同步修改代理上游；本地 HTTP 模式也需将 `AGENTX_PUBLIC_URL` 设为含新端口的完整地址。部署和接入步骤见[管理与 token 操作](../.docs/architecture/cloud-scope.md)和[主机登记](../.docs/architecture/cloud-daemon.md)。

## 从源码交付

在 Ubuntu 24.04 构建机上准备 Docker、项目指定的 Rust 工具链、Node 和 just，在仓库根目录执行：

```sh
npm ci --prefix packages/agentx-client
npm ci
npm ci --prefix web/inspector
just cloud-image
bash deploy/smoke.sh
```

生成的本地镜像名为 `agentx-cloud:local`，也是 Compose 的默认值。已有同架构、兼容 Ubuntu 24.04 的可执行文件时，可运行 `bash deploy/build-image.sh /path/to/agentx my-agentx:v1`。构建上下文仅含可执行文件、许可证和 Dockerfile。离线交付使用 `docker save agentx-cloud:local | gzip > agentx-cloud.tar.gz`，目标机执行 `docker load -i agentx-cloud.tar.gz`，再用同一份 Compose 启动。

[GitHub 镜像工作流](../.github/workflows/cloud-image.yml)在相关 PR 或手动触发时构建并验证镜像。推送与程序版本一致的 `vX.Y.Z` 标签后，发布作业使用仓库的 `GITHUB_TOKEN` 将通过验证的镜像推送至 `ghcr.io/<owner>/<repo>-cloud:vX.Y.Z`。首次公开交付时，在 Packages 中核对包的可见性。冒烟检查使用独立项目、随机 token 和临时端口，覆盖 Web 入口、未授权访问、Admin 创建 scope 和容器重建后的数据读取。

在既有 Release 上补发 GHCR 镜像时，使用 `gh workflow run cloud-image.yml --ref main -f release_tag=vX.Y.Z`。工作流核对 Release 中镜像归档的校验和与程序版本，再用该标签对应的冒烟测试验证镜像。
