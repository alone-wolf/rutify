# 问题 ID
Q20260301-01

# 当前状态
waiting_user

# 最后更新时间
2026-03-01 21:46 +08:00

# 问题标题
/api/notifies 删除接口未做鉴权，存在未授权删除风险

# 问题摘要
API删除接口缺少鉴权

# 问题描述
当前 `DELETE /api/notifies` 与 `DELETE /api/notifies/{id}` 可在未携带任何认证信息时直接执行数据库删除操作。与同项目 `/auth/tokens` 等受保护接口相比，删除能力暴露范围过大，属于高风险越权行为。

# 严重程度
High

# 影响对象
- 服务端模块：`rutify-server` 路由层
- 接口：`/api/notifies`、`/api/notifies/{id}`
- 使用方：所有可访问服务端 HTTP 端口的调用方

# 问题原因
`routes/api/notifies.rs` 中直接注册了 `DELETE` 路由并执行删除逻辑；`routes/api/mod.rs` 未对该路由组施加认证中间件。当前鉴权中间件只在 `routes/auth.rs` 的 `protected_router` 中生效，未覆盖 `/api/notifies` 删除接口。

# 核心证据路径
- `packages/rutify-server/src/routes/api/notifies.rs`
- `packages/rutify-server/src/routes/api/mod.rs`

# 待确认差异
无

# 造成问题的证据
- 代码路径：
  - `packages/rutify-server/src/routes/api/notifies.rs` 中 `delete_all_notifies_handler` / `delete_notify_by_id_handler` 直接执行 `Entity::delete_many` / `Entity::delete_by_id`。
  - `packages/rutify-server/src/routes/api/mod.rs` 仅做 `nest`，无鉴权层。
  - `packages/rutify-server/src/routes/auth.rs` 显示项目内已有 `user_auth_middleware` 的受保护路由范式。
- 日志/报错：暂无（静态审查阶段）。
- 配置位置：无额外配置可限制该删除行为。
- 复现步骤：
  1. 启动服务端。
  2. 不带 `Authorization` 请求头调用 `DELETE /api/notifies`。
  3. 当前实现下可直接返回成功并删除记录（未出现 401）。

# 影响
- 安全：未授权调用方可删除通知数据。
- 功能：可导致运维/用户历史通知丢失。
- 可维护性：接口权限模型不一致，后续审计难度增大。

# 建议解决方案
1. 将 `/api/notifies` 两个 `DELETE` 路由纳入 `user_auth_middleware` 保护。
2. 保持 `GET /api/notifies` 的访问策略显式化（按产品要求决定是否也需鉴权），并在代码中分离公开路由与受保护路由。
3. 在 README 或 API 文档中补充删除接口鉴权要求，避免调用方误用。

# 验收标准
1. 未携带有效用户 JWT 调用 `DELETE /api/notifies` 与 `DELETE /api/notifies/{id}` 时，HTTP 状态码为 `401`。
2. 携带有效用户 JWT 调用上述删除接口时，仍可成功删除并返回 `200`。
3. 相关接口权限要求在项目文档中有明确说明，且路径与参数描述与代码一致。

# 验证记录
- 标准 1 -> 验证动作：N/A（待修复后执行接口回归） | 结果：N/A | 证据/原因：当前处于问题登记阶段，尚未进入修复。
- 标准 2 -> 验证动作：N/A（待修复后执行接口回归） | 结果：N/A | 证据/原因：当前处于问题登记阶段，尚未进入修复。
- 标准 3 -> 验证动作：N/A（待修复后执行文档核对） | 结果：N/A | 证据/原因：当前处于问题登记阶段，尚未进入修复。

# 评论
（由用户填写：同意修复 / 修改方案 / 暂不处理 / 拒绝）

# 状态变更记录
- 时间：2026-03-01 21:46 +08:00 | 状态：waiting_user | 原因：新建问题并等待用户确认 | 操作者：codex | 关联提交：N/A
- 时间：YYYY-MM-DD HH:MM +00:00 | 状态： | 原因： | 操作者： | 关联提交：
- 时间：YYYY-MM-DD HH:MM +00:00 | 状态： | 原因： | 操作者： | 关联提交：
