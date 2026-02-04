# Rutify SDK

Rutify SDK 是一个用于与 Rutify 服务器交互的 Rust 客户端库。

## 功能特性

- 🚀 简单易用的 API
- 🔔 发送通知
- 📊 获取服务器统计信息
- 📬 获取通知列表
- ⚡ 异步支持
- 🛡️ 完善的错误处理

## 快速开始

### 在您的项目中使用 SDK

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
rutify = { path = "/path/to/rutify" }
tokio = { version = "1.0", features = ["full"] }
```

### 基本用法

```rust
use rutify::sdk::{RutifyClient, NotificationInput};
use tokio;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建客户端
    let client = RutifyClient::new("http://127.0.0.1:3000");
    
    // 发送通知
    let notification = NotificationInput {
        notify: "Hello from SDK!".to_string(),
        title: Some("Test Notification".to_string()),
        device: Some("my-device".to_string()),
    };
    
    client.send_notify(&notification).await?;
    println!("通知发送成功！");
    
    // 获取统计信息
    let stats = client.get_stats().await?;
    println!("服务器统计: {:?}", stats);
    
    // 获取通知列表
    let notifies = client.get_notifies().await?;
    println!("通知数量: {}", notifies.len());
    
    Ok(())
}
```

## 命令行客户端

项目包含一个功能完整的命令行客户端，使用现代的 clap derive 宏实现：

### 获取帮助

```bash
cargo run --bin client -- --help
```

### 发送通知

```bash
# 基本通知
cargo run --bin client -- send --message "Hello World"

# 带标题和设备的通知
cargo run --bin client -- send \
  --message "Server started" \
  --title "System Alert" \
  --device "web-server"

# 使用短参数
cargo run --bin client -- send -m "Test" -t "Title" -d "device"
```

### 获取统计信息

```bash
cargo run --bin client -- stats
```

### 获取通知列表

```bash
cargo run --bin client -- notifies
```

### 指定服务器地址

```bash
cargo run --bin client -- --server http://192.168.1.100:3000 stats
```

### CLI 特性

- ✅ 使用 clap derive 宏实现类型安全的参数解析
- ✅ 自动生成帮助信息和错误提示
- ✅ 支持短参数和长参数
- ✅ 默认值支持
- ✅ 子命令结构

## API 参考

### RutifyClient

主要的客户端类，提供所有 API 方法。

#### 方法

- `new(base_url: impl Into<String>) -> Self`: 创建新客户端
- `with_timeout(base_url: impl Into<String>, timeout: Duration) -> SdkResult<Self>`: 创建带超时的客户端
- `get_notifies(&self) -> SdkResult<Vec<NotifyItem>>`: 获取所有通知
- `get_stats(&self) -> SdkResult<Stats>`: 获取服务器统计信息
- `send_notify(&self, input: &NotificationInput) -> SdkResult<()>`: 发送通知
- `send_notify_get(&self, input: &NotificationInput) -> SdkResult<()>`: 通过 GET 发送通知
- `send_notify_post(&self, input: &NotificationInput) -> SdkResult<()>`: 通过 POST 发送通知
- `connect_websocket<F>(&self, callback: F) -> SdkResult<()>`: 连接 WebSocket 并监听通知
- `disconnect_websocket(&self) -> SdkResult<()>`: 断开 WebSocket 连接
- `is_websocket_connected(&self) -> bool`: 检查 WebSocket 连接状态
- `send_websocket_message(&self, message: &str) -> SdkResult<()>`: 通过 WebSocket 发送消息

### 类型定义

#### NotificationInput

发送通知的输入结构：

```rust
pub struct NotificationInput {
    pub notify: String,        // 通知内容（必需）
    pub title: Option<String>, // 通知标题（可选）
    pub device: Option<String>, // 设备名称（可选）
}
```

#### NotifyItem

通知项目结构：

```rust
pub struct NotifyItem {
    pub id: i32,
    pub title: String,
    pub notify: String,
    pub device: String,
    pub received_at: String,
}
```

#### Stats

服务器统计信息：

```rust
pub struct Stats {
    pub today_count: i32,
    pub total_count: i32,
    pub device_count: i32,
    pub is_running: bool,
}
```

#### WebSocket 消息类型

WebSocket 消息枚举：

```rust
pub enum NotificationMessage {
    Event(NotifyEvent),    // 结构化通知事件
    Text(String),          // 纯文本消息
    Close,                 // 连接关闭
    Error(String),         // 错误信息
}
```

### WebSocket 功能

SDK 提供了完整的 WebSocket 支持，可以实时监听通知：

```rust
use rutify::sdk::{RutifyClient, NotificationMessage};

let client = RutifyClient::new("http://127.0.0.1:3000");

// 连接 WebSocket 并监听通知
client.connect_websocket(|msg| {
    match msg {
        NotificationMessage::Event(event) => {
            println!("收到通知: {}", event.data.notify);
        }
        NotificationMessage::Text(text) => {
            println!("收到文本: {}", text);
        }
        NotificationMessage::Close => {
            println!("连接已关闭");
        }
        NotificationMessage::Error(err) => {
            println!("错误: {}", err);
        }
    }
}).await?;

// 检查连接状态
if client.is_websocket_connected().await {
    println!("WebSocket 已连接");
}

// 发送消息
client.send_websocket_message("Hello WebSocket!").await?;

// 断开连接
client.disconnect_websocket().await?;
```

### 错误处理

SDK 使用 `SdkError` 枚举来处理各种错误情况：

```rust
pub enum SdkError {
    HttpError(reqwest::Error),
    JsonError(serde_json::Error),
    ApiError { status: String },
    InvalidUrl(url::ParseError),
    NetworkError(String),
}

pub type SdkResult<T> = Result<T, SdkError>;
```

## WebSocket 应用

项目包含一个专门的 WebSocket 应用示例：

### 监听通知

```bash
# 启动监听模式
cargo run --bin application -- listen
```

### 发送通知并监听响应

```bash
# 发送通知并监听响应
cargo run --bin application -- send-and-listen --message "Hello WebSocket!" --title "Test"

# 使用短参数
cargo run --bin application -- send-and-listen -m "Test message" -t "Title" -d "device"
```

### WebSocket 应用特性

- ✅ 实时通知监听
- ✅ 发送通知并接收确认
- ✅ 自动连接管理
- ✅ 错误处理和重连
- ✅ 友好的控制台输出

## 示例项目

查看 `src/bin/client.rs` 获取完整的命令行客户端实现示例。
查看 `src/bin/application.rs` 获取 WebSocket 应用实现示例。
