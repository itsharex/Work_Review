# Report Fallback Header Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将报告页头改成三层文本结构，并把 AI 回退原因从后端保存到前端展示。

**Architecture:** 后端为生成结果和日报存储增加 `fallback_reason` 字段，并在 AI 回退时写入友好原因；前端 `reportMeta` 基于该字段生成页头文案，报告页用更简洁的文本层级替换当前的多块状态展示。

**Tech Stack:** Rust、rusqlite、Svelte 4、Node `node:test`

---

### Task 1: 后端日报回退原因链路

**Files:**
- Modify: `src-tauri/src/analysis/mod.rs`
- Modify: `src-tauri/src/analysis/summary.rs`
- Modify: `src-tauri/src/analysis/local.rs`
- Modify: `src-tauri/src/analysis/cloud.rs`
- Modify: `src-tauri/src/database.rs`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: 写失败测试**
- [ ] **Step 2: 运行测试确认失败**
- [ ] **Step 3: 实现 fallback_reason 保存与读取**
- [ ] **Step 4: 运行 Rust 测试确认通过**

### Task 2: 报告页头简化与文案映射

**Files:**
- Modify: `src/routes/report/reportMeta.js`
- Modify: `src/routes/report/reportMeta.test.js`
- Modify: `src/routes/report/Report.svelte`
- Modify: `src/routes/report/ReportLayout.test.js`
- Modify: `src/app.css`
- Modify: `src/lib/i18n/index.js`

- [ ] **Step 1: 写失败测试**
- [ ] **Step 2: 运行测试确认失败**
- [ ] **Step 3: 实现三层文本页头结构**
- [ ] **Step 4: 运行前端测试确认通过**
