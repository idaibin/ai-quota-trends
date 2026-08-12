下面是可直接交给 Codex 执行的完整 Markdown 规格。已调整为参考 `rustzen-tools/products/clear` 的工程思想：

* Local-first
* Rust-first
* SQLite-first
* Tauri 产品化
* 简单边界
* 可验证交付
* 不过度设计

保存建议：

```text
docs/ai-quota-trends-spec.md
```

````markdown
# AI Quota Trends 产品与实施规格

## 1. 项目概述

项目名称：

AI Quota Trends

仓库：

ai-quota-trends


## 1.1 产品定位

AI Quota Trends 是一个 macOS 本地优先（Local-first）的多 AI 服务额度与 Token 使用仪表盘。当前持续采集 Codex 额度与 Token 活动，只读汇总 ZCode 本地 Token 记录，并通过已安装 CLI 的额度界面读取 Qoder 国内版和 Antigravity 当前额度。

菜单栏标题仍固定使用 Codex 额度。弹出窗一次完整展示所有已启用工具，不提供切换器；Codex 展示持久化额度窗口，Qoder 国内版展示套餐与 Credit，Antigravity 展示两个共享模型池。ZCode 当前只展示已接入 Token 明细，不虚构账户额度。弹出窗不展示 Codex 趋势图。

Token 热力图以本地已完成请求的 total Token 为统一口径，汇总 Codex 与可验证的本地工具记录。每日悬停仅显示日期与 provider subtotal，不重复当天总量，也不展示 model 子行；可见 provider subtotal 与当天总量对账。

核心目标：

> 持续采集 Codex 当前额度状态，记录额度变化历史，分析消耗速度，并提供本地趋势可视化。


它不是：

- Codex 客户端
- AI 助手
- Token 成本分析工具
- 云端管理平台


它解决的问题：

1. 当前 Codex 额度还有多少？
2. 本周额度下降速度如何？
3. 什么时候发生了额度消耗？
4. 是否存在异常快速消耗？
5. 额度什么时候恢复？


---

# 2. 设计原则

参考 `rustzen-tools/products/clear` 的产品设计原则：

## 2.1 Local-first

所有数据默认保存在本机。

禁止：

- 云端同步
- 用户账号系统
- 数据上传
- 第三方统计


---

## 2.2 Rust-first

核心能力全部由 Rust 实现。

包括：

- Codex app-server 通信
- 数据采集
- 数据分析
- SQLite 存储
- 后台任务


前端只负责：

- 展示
- 交互
- 图表


---

## 2.3 SQLite-first

采用 SQLite 作为唯一数据存储。


禁止引入：

- PostgreSQL
- Redis
- Kafka
- 云数据库


原因：

这是个人桌面工具，不需要复杂基础设施。


---

## 2.4 Simple-first

避免：

- 过度抽象
- 多层框架
- 插件系统
- 复杂事件总线


优先：

简单、稳定、可维护。


---

# 3. 技术架构


## 3.1 技术栈


桌面：

Tauri 2


Backend：

Rust


Runtime：

Tokio


Database：

SQLite


Frontend：

React + TypeScript


Build：

Vite + Bun


Charts：

轻量图表库


---

# 4. 项目结构


```text
ai-quota-trends

├── src/
│
│   ├── App.tsx
│   │
│   ├── pages/
│   │   ├── Overview.tsx
│   │   ├── Trends.tsx
│   │   ├── Activity.tsx
│   │   ├── Alerts.tsx
│   │   └── Settings.tsx
│
│   ├── components/
│   │
│   └── api/
│
│
├── src-tauri/
│
│   └── src/
│
│       ├── main.rs
│       ├── app.rs
│       ├── commands.rs
│
│       ├── codex/
│       │
│       │   ├── app_server.rs
│       │   ├── rpc.rs
│       │   ├── protocol.rs
│       │   └── reconnect.rs
│
│       ├── quota/
│       │
│       │   ├── collector.rs
│       │   ├── model.rs
│       │   ├── analyzer.rs
│       │   └── alert.rs
│
│       ├── storage/
│       │
│       │   ├── sqlite.rs
│       │   ├── migration.rs
│       │   └── repository.rs
│
│
├── docs/
│
│   ├── architecture.md
│   ├── database.md
│   └── verification.md
│
└── README.md
````

---

# 5. Codex 数据来源

## 5.1 数据来源

使用：

```
codex app-server
```

禁止：

直接调用：

```
chatgpt.com/backend-api/*
```

禁止：

读取：

```
~/.codex/auth.json
```

原因：

认证交给 Codex CLI。

---

# 6. Codex App Server

## 6.1 启动

执行：

```bash
codex app-server --listen stdio://
```

Rust：

使用：

```rust
tokio::process::Command
```

---

## 6.2 初始化流程

发送：

```json
{
 "method":"initialize"
}
```

随后：

```json
{
 "method":"initialized"
}
```

---

## 6.3 支持 RPC

必须支持：

```
account/read

account/rateLimits/read
```

---

## 6.4 支持事件

监听：

```
account/rateLimits/updated
```

收到事件：

不要直接使用事件数据。

流程：

```
event received

↓

account/rateLimits/read

↓

normalize

↓

save snapshot
```

原因：

事件可能只是增量通知。

---

# 7. 数据模型

## 7.1 Quota

禁止：

```rust
struct Quota {

 five_hour: Window,

 weekly: Window

}
```

原因：

Codex 额度窗口可能变化。

必须：

```rust
struct QuotaSnapshot {

 limit_id: String,

 windows: Vec<QuotaWindow>

}


struct QuotaWindow {

 window_minutes: Option<u64>,

 used_percent: f64,

 reset_at: Option<i64>

}
```

---

# 8. SQLite

## quota_snapshots

```sql
CREATE TABLE quota_snapshots(

id INTEGER PRIMARY KEY,

created_at INTEGER NOT NULL,

limit_id TEXT,

window_minutes INTEGER,

used_percent REAL,

reset_at INTEGER,

raw_json TEXT

);
```

用途：

保存历史额度变化。

---

## collector_events

```sql
CREATE TABLE collector_events(

id INTEGER PRIMARY KEY,

created_at INTEGER NOT NULL,

event_type TEXT,

message TEXT

);
```

事件：

```
connected

disconnected

reconnected

quota_changed

schema_changed
```

---

# 9. 额度采集服务

## 生命周期

```
Application Start

↓

Start Collector

↓

Start app-server

↓

Initialize

↓

Read quota

↓

Save snapshot

↓

Listen event

↓

60s polling fallback

```

---

# 10. 采集策略

## Event 优先

来源：

```
account/rateLimits/updated
```

---

## Poll 兜底

周期：

60 秒

执行：

```
account/rateLimits/read
```

---

## 保存策略

只有变化才写入。

比较：

```
used_percent

reset_at

window_minutes
```

---

# 11. 趋势分析

## 11.1 当前状态

显示：

```
Remaining

Used

Reset Time

Last Updated
```

---

## 11.2 消耗速度

计算：

过去：

```
15 minutes

1 hour

24 hours
```

公式：

```
(delta used percent)

/ elapsed time
```

---

## 11.3 使用节奏

计算：

```
周期时间进度

vs

额度消耗进度
```

示例：

```
周期过去 40%

额度消耗 65%

状态:
Above pace
```

---

# 12. UI

## Overview

显示：

* 当前额度
* 剩余比例
* 重置时间
* 数据更新时间

---

## Trends

显示：

* 24小时曲线
* 当前周期曲线
* 消耗速度

---

## Alerts

显示：

* 快速下降
* 重置
* 数据异常

---

## Settings

The Settings UI does not display or edit a Codex executable path. For backward
compatibility, a non-empty persisted `codexPath` value remains an internal
collector override; empty values use `PATH` first, then Volta/local and Homebrew
locations. The provider catalog displays status and version only.
The General group contains collection frequency, launch-at-login, menu-bar-only,
and theme controls; the obsolete floating-window trend-range control is not shown.
Models & Tools is a compact single-column catalog. Codex is the primary read-only
source and ZCode/Qoder CN/Antigravity keep enable toggles.

配置：

* Poll interval
* Alert threshold
* Launch startup
* Data retention

---

# 13. 异常检测

## Rapid Drain

规则：

默认：

```
10分钟下降 >=5%
```

触发：

```
Quota dropped quickly
```

---

## Collector Offline

规则：

超过：

```
5分钟无数据
```

状态：

```
Collector stale
```

---

## Reset

检测：

```
used_percent

大幅下降
```

记录：

```
Quota reset detected
```

---

# 14. macOS 集成

实现：

* Menu Bar
* Notification
* Auto Start

不要：

* Dock 主应用
* 常驻窗口

目标：

类似：

iStat Menus

---

# 15. 验证标准

每个功能必须完成：

## Code

完成实现。

## Test

增加测试。

## Docs

更新文档。

## Verify

提供验证结果。

---

# Phase 1

## 工程初始化

完成：

* Tauri运行
* SQLite初始化

验证：

```bash
pnpm tauri dev
```

结果：

应用启动。

---

# Phase 2

## App Server

完成：

* 启动
* RPC
* quota读取

验证：

输出：

```
Current quota snapshot
```

---

# Phase 3

## Collector

完成：

* Event
* Poll
* Storage

验证：

SQLite出现历史记录。

---

# Phase 4

## Trends

完成：

* 曲线
* 消耗速度

验证：

模拟数据生成正确趋势。

---

# Phase 5

## UI

完成：

* Menu bar
* Dashboard

验证：

人工检查。

---

# Phase 6

## Release

完成：

* macOS build
* install test

验证：

clean machine运行。

---

# 16. Git 分支策略

每个阶段独立：

```
feat/bootstrap

feat/app-server

feat/quota-collector

feat/sqlite-storage

feat/trend-analysis

feat/menu-ui

feat/alerts

release/v1
```

---

# 17. 第一版边界

必须：

✅ Tauri

✅ Rust backend

✅ app-server

✅ quota snapshot

✅ SQLite

✅ trend chart

✅ alert

不做：

❌ 云同步

❌ 登录系统

❌ AI分析

❌ 精确费用计算

❌ Codex替代客户端

---

# 最终目标

构建一个：

> 本地优先、低资源占用、macOS 原生体验的 AI Quota Trends 用量观察器。

核心指标：

稳定采集。

准确记录。

清晰趋势。

可验证行为。

```

这个版本更适合作为 Codex 的执行规格，而不是产品介绍文档。它把实现边界、技术约束、验证方式和阶段拆分都固定下来。
```
