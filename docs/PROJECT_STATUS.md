# Project Status

## Baseline (2026-02-28)

项目已完成一次收敛清理，目标是把代码和文档恢复到可维护状态。

## 已完成

- 删除 `rutify-server` 中未落地且未引用的空壳模块树：
  - `application/`
  - `domain/`
  - `infrastructure/`
  - `interface/`
  - `security/`
- 合并 Token 数据访问逻辑，移除重复文件 `db/token_ops_new.rs`。
- `/api/notifies`、`/api/stats` 从硬编码假数据切换为真实数据库读取。
- `rutify-sdk` 的 token 创建路径与服务端路由统一为 `/auth/tokens`。
- CLI 默认服务器地址统一为 `http://127.0.0.1:3000`。
- 根目录阶段性报告/临时文档已清理，`README.md` 重写为单一入口。

## 现在的稳定边界

- 服务端稳定入口：`routes/` + `services/auth/` + `db/`
- 客户端稳定入口：`rutify-sdk` + `rutify-client` + `rutify-cli`
- UI 包（`rutify-application`、`rutify-panel`）仍属于迭代区

## 待办建议

- 增加最小集成测试（`/notify`、`/api/notifies`、`/api/stats`、`/auth/login`）。
- 在 CI 中固定 `cargo fmt --check` 和 `cargo check`。
- 为认证流程补充端到端示例（注册 -> 登录 -> 创建 notify token -> WS 订阅）。
