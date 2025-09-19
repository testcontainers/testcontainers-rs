## PRD: 在容器内访问宿主机端口（host.testcontainers.internal）

参考与背景：见 GitHub 议题 “Support for Host Port Exposure (host.testcontainers.internal)” [#821](https://github.com/testcontainers/testcontainers-rs/issues/821)。

### 背景与目标

- 容器需要以统一主机名访问宿主机端口，降低测试网络配置复杂度。
- 提供最小 API：仅声明需要访问的宿主端口，其余自动完成。
- 实现路径唯一：SSHD 侧车 + SSH 反向端口转发（基于 ssh2）。

### 范围与非目标

- 不覆盖用户显式的 `with_host(...)` 设置。
- 首版仅支持 TCP；不覆盖跨主机编排场景。

---

## 能力与配置

- 固定主机名别名：`host.testcontainers.internal`（不可配置）。
- 固定侧车镜像：`testcontainers/sshd:1.3.0`。
- API（增量）：
  - `with_exposed_host_port(port: u16)`
  - `with_exposed_host_ports(ports: impl IntoIterator<Item = u16>)`

### 特性开关与依赖策略（避免与 rustls 冲突）

- 默认不启用 host 暴露功能，避免引入 OpenSSL。
- Features（示例）：
  - `host-expose`：启用“宿主端口访问”并打开可选依赖 `ssh2`。
  - `host-expose-vendored-openssl`：在 `host-expose` 基础上启用 `ssh2/vendored-openssl`，静态捆绑 OpenSSL。
- Cargo.toml（示意）：

```toml
[features]
default = []
host-expose = ["ssh2"]
host-expose-vendored-openssl = ["host-expose", "ssh2/vendored-openssl"]

[dependencies]
ssh2 = { version = "0.9", optional = true }
```

- 说明：`ssh2` 依赖 OpenSSL（支持 vendored 方案）。仅当 `host-expose` 启用时编译相关代码。

---

## 技术实现

### 拓扑与数据流

- 侧车（sshd）与被测容器处于同一用户网络。
- 被测容器的 `/etc/hosts` 注入：`host.testcontainers.internal` → 侧车容器 IP。
- 宿主建立到侧车的 SSH 会话，并在“远端（侧车）”监听 `<remote_port>`，将流量经 SSH 通道回送到宿主 `127.0.0.1:<host_port>`（等价于 `ssh -R 0.0.0.0:<remote_port>:127.0.0.1:<host_port>`）。
- 默认同号映射：`remote_port == host_port`，容器内可直接访问 `host.testcontainers.internal:<host_port>`。

### sshd 要求（侧车镜像）

- `AllowTcpForwarding yes`
- `GatewayPorts clientspecified`（允许远端转发绑定到 0.0.0.0）
- 仅公钥登录（禁用密码）

### 基于 ssh2 的实现（关键步骤）

- 会话建立：
  - `Session::new()` → `set_tcp_stream(TcpStream::connect("127.0.0.1:<host_ssh_port>"))` → `handshake()` → `userauth_pubkey_memory()`。
  - 可启用 keepalive：`session.set_keepalive(true, 10)`。
- 远端监听（等价于 -R）：
  - `let (listener, bound_port) = session.channel_forward_listen(remote_port, Some("0.0.0.0"), None)?;`
  - 可传 `remote_port = 0` 让服务器分配端口（使用 `bound_port`）。参见 ssh2 文档：`https://docs.rs/ssh2/latest/ssh2/struct.Session.html#method.channel_forward_listen`。
- 接入与桥接：
  - 循环 `listener.accept()` 获得入站 `Channel`；为每个连接建立到宿主 `127.0.0.1:<host_port>` 的 TCP 连接。
  - 双向拷贝：`Channel.read ↔ HostTcp.write`，`HostTcp.read ↔ Channel.write`。
  - 并发与阻塞：`ssh2` 为同步 I/O，可用线程池或 `spawn_blocking` 包装数据泵；连接数量通常有限。
- 生命周期与清理：
  - 记录侧车容器 ID 与 SSH 会话句柄；容器结束即关闭会话、销毁侧车与临时网络。
  - 异常断开时，监听循环退出并上抛错误，便于测试察觉。

---

## 代码改动落点

- `testcontainers/src/core/containers/request.rs`
  - 新增：`host_port_exposures: Option<Vec<u16>>`。
- `testcontainers/src/core/image/image_ext.rs`
  - 新增：`with_exposed_host_port`、`with_exposed_host_ports`。
- `testcontainers/src/runners/{async_runner,sync_runner}.rs`
  - 创建/复用网络；启动侧车（`testcontainers/sshd:1.3.0`）。
  - 生成一次性密钥，注入 `authorized_keys`；建立 SSH 会话与反向转发（多端口复用单会话，多个 `-R`）。
  - 注入 hosts：`host.testcontainers.internal` → 侧车 IP。
  - 生命周期：侧车容器与 SSH 会话随主容器清理。

---

## 运行时流程

1. 解析配置（API 端口清单）。
2. 准备网络与侧车；注入密钥；宿主建立 SSH 会话与反向端口转发；写入 hosts。
3. 启动被测容器，进入既有 wait/inspect 流程。
4. 清理：关闭 SSH 会话，停止侧车与临时网络（按复用策略）。

---

## 错误处理与可观测性

- 记录事件：侧车启动/就绪、会话建立、远端监听、桥接错误、清理动作。
- 失败即报错：在被测容器启动前抛出明确错误（含端口与会话信息）。

---

## 测试

- 集成测试：
  - 宿主起临时 HTTP 服务（随机端口）；容器内 `curl http://host.testcontainers.internal:<port>` 返回 200。
  - 断开 SSH 会话后访问失败（验证清理与生命周期）。
- 单元测试：
  - API 解析（端口清单）。
  - hosts 注入不覆盖用户显式设置。
  - 侧车生命周期管理（创建/清理）。

---

## 性能与安全

- 额外 1 个侧车容器 + 1 条 SSH 会话；仅在声明端口时启用。
- 禁用密码、一次性密钥、仅开启必要反向转发；会话随测试生命周期清理。

---

## 风险与替代

- 受限环境可能阻断 SSH/转发；文档提供手动网络与别名配置作为替代路径。

---

## 验收标准（DoD）

- 启用 `host-expose` 后示例用例通过；文档包含排错与常见问题。
- API 与项目风格一致；lint/CI 通过。
