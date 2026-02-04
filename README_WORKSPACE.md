# Rutify Workspace

一个基于 Rust 的通知系统，采用 workspace 架构，包含 SDK、服务器、CLI 客户端和 GUI 应用。

## 📦 包结构

```
rutify-workspace/
├── packages/
│   ├── rutify-sdk/          # 核心 SDK 库
│   ├── rutify-server/       # 服务器应用
│   ├── rutify-client/       # CLI 客户端
│   └── rutify-ui/           # WebSocket 应用
├── Cargo.toml               # Workspace 配置
└── README.md
```

## 🚀 快速开始

### 构建所有包
```bash
cargo build --workspace
```

### 运行服务器
```bash
cargo run --package rutify-server -- --ui
```

### 运行 CLI 客户端
```bash
# 获取通知
cargo run --package rutify-client -- notifies

# 获取统计
cargo run --package rutify-client -- stats

# 发送通知
cargo run --package rutify-client -- send "Hello World" --title "Test" --device "my-device"
```

### 运行 WebSocket 应用
```bash
# 监听通知
cargo run --package rutify-ui -- listen

# 发送并监听
cargo run --package rutify-ui -- send-and-listen --message "Hello" --title "Test"
```

## 📋 包说明

### rutify-sdk
核心 SDK 库，提供 HTTP 和 WebSocket 客户端功能。

**主要功能：**
- HTTP API 客户端
- WebSocket 连接管理
- 类型安全的 API 响应
- 统一的错误处理

### rutify-server
通知服务器，提供 REST API 和 WebSocket 服务。

**主要功能：**
- RESTful API
- WebSocket 实时通知
- SQLite 数据库存储
- Slint GUI 界面

### rutify-client
命令行客户端，用于与服务器交互。

**主要功能：**
- 获取通知列表
- 获取服务器统计
- 发送新通知

### rutify-ui
WebSocket 应用，用于实时监听和发送通知。

**主要功能：**
- 实时通知监听
- WebSocket 消息发送
- 命令行界面

## 🛠️ 开发

### 单独构建某个包
```bash
cargo build --package rutify-sdk
cargo build --package rutify-server
cargo build --package rutify-client
cargo build --package rutify-ui
```

### 运行测试
```bash
cargo test --workspace
```

### 检查代码
```bash
cargo check --workspace
```

## 📝 依赖管理

Workspace 使用统一的依赖版本管理，所有包共享相同的依赖版本，确保兼容性。

主要依赖：
- `tokio`: 异步运行时
- `axum`: Web 框架
- `sea-orm`: ORM 框架
- `slint`: GUI 框架
- `clap`: CLI 框架
- `reqwest`: HTTP 客户端

## 🔄 迁移说明

此项目已从单体结构迁移到 workspace 架构：

- **之前**: 单一 `Cargo.toml`，所有代码在 `src/` 目录
- **现在**: 多包结构，每个包独立管理

迁移优势：
- 更好的关注点分离
- 独立的版本管理
- 更快的编译速度
- 更清晰的依赖关系
