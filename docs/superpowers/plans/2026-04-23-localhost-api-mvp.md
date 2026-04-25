# Localhost API MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 为 Work Review 桌面端增加默认关闭的 `127.0.0.1` 本地 API MVP，支持当前设备的日报生成、日报读取、Markdown 导出与节点状态查询。

**Architecture:** 在现有桌面端内新增一个轻量 `Node Gateway` 模块，复用现有日报命令与配置持久化能力；本地 API 默认关闭，通过设置页显式开启，并使用独立 token 鉴权。首版仅支持本机 HTTP/JSON，不引入控制面或远程同步。

**Tech Stack:** Rust、Tauri 2、tokio、Svelte 4、Node test、Rust unit test

---

### Task 1: 配置与设置页骨架

**Files:**
- Modify: `src-tauri/src/config.rs`
- Modify: `src/routes/settings/components/SettingsGeneral.svelte`
- Modify: `src/lib/i18n/index.js`
- Test: `src/routes/settings/SettingsGeneral.test.js`

- [ ] **Step 1: 先写失败测试，锁定设置页与配置字段**
- [ ] **Step 2: 运行前端测试，确认新增断言先失败**
- [ ] **Step 3: 增加本地 API 配置字段与默认值**
- [ ] **Step 4: 在基本设置页加入开关、端口、token 管理文案与按钮**
- [ ] **Step 5: 重新运行前端测试，确认通过**

### Task 2: Node Gateway 本地服务

**Files:**
- Create: `src-tauri/src/localhost_api.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/error.rs`
- Test: `src-tauri/src/localhost_api.rs`

- [ ] **Step 1: 先写 Rust 单元测试，锁定 token 解析、鉴权和最小路由语义**
- [ ] **Step 2: 运行对应 Rust 测试，确认先失败**
- [ ] **Step 3: 实现本地 token 存储、HTTP 解析、响应封装与最小服务生命周期**
- [ ] **Step 4: 在应用启动与配置变更时同步启停本地服务**
- [ ] **Step 5: 重新运行 Rust 测试，确认通过**

### Task 3: Tauri 命令与复用日报链路

**Files:**
- Modify: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/localhost_api.rs`
- Test: `src-tauri/src/commands.rs`

- [ ] **Step 1: 先写失败测试，锁定本地 API 状态查询、token 轮换与节点状态结构**
- [ ] **Step 2: 运行对应 Rust 测试，确认先失败**
- [ ] **Step 3: 增加状态查询与 token 轮换命令，并让本地 API 复用现有日报生成/读取/导出逻辑**
- [ ] **Step 4: 重新运行 Rust 测试，确认通过**

### Task 4: 验证

**Files:**
- Modify: `README.md`（如实现时必须补充说明）

- [ ] **Step 1: 运行相关 Node 测试**
- [ ] **Step 2: 运行相关 Rust 测试**
- [ ] **Step 3: 如可行，运行一次构建级验证**
- [ ] **Step 4: 记录实际验证结果与剩余风险**
