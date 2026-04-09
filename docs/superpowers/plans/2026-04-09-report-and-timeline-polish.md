# Report And Timeline Polish Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 修复报告页遮挡与元信息歧义，优化时间线详情图打开延迟，并增强时段摘要的信息层次。

**Architecture:** 将报告页元信息解析提取为可测试 helper；报告页与时段摘要只做前端结构与样式增强；时间线详情打开流程改为前端并行请求，不改后端接口。整个改动保持在现有 Svelte 页面与轻量 helper 范围内。

**Tech Stack:** Svelte 4、Node `node:test`、Tauri invoke、现有 `app.css`

---

### Task 1: 报告页元信息与响应式布局

**Files:**
- Create: `src/routes/report/reportMeta.js`
- Create: `src/routes/report/reportMeta.test.js`
- Modify: `src/routes/report/Report.svelte`
- Modify: `src/routes/report/ReportLayout.test.js`
- Modify: `src/lib/i18n/index.js`
- Modify: `src/app.css`

- [ ] **Step 1: 写失败测试**
- [ ] **Step 2: 运行测试确认失败**
- [ ] **Step 3: 实现元信息 helper 与报告页布局修复**
- [ ] **Step 4: 运行相关测试确认通过**

### Task 2: 时间线详情图并行加载

**Files:**
- Modify: `src/routes/timeline/Timeline.svelte`
- Modify: `src/routes/timeline/TimelineLayout.test.js`

- [ ] **Step 1: 写失败测试**
- [ ] **Step 2: 运行测试确认失败**
- [ ] **Step 3: 实现并行请求与缩略图占位**
- [ ] **Step 4: 运行相关测试确认通过**

### Task 3: 时段摘要信息增强

**Files:**
- Create: `src/routes/timeline/summaryPresentation.js`
- Create: `src/routes/timeline/summaryPresentation.test.js`
- Modify: `src/routes/timeline/Summary.svelte`
- Modify: `src/routes/timeline/SummaryLayout.test.js`
- Modify: `src/lib/i18n/index.js`

- [ ] **Step 1: 写失败测试**
- [ ] **Step 2: 运行测试确认失败**
- [ ] **Step 3: 实现副摘要、节奏信息与增强布局**
- [ ] **Step 4: 运行相关测试确认通过**

### Task 4: 回归验证

**Files:**
- Modify: `src/routes/report/ReportEditorial.test.js`（如需）

- [ ] **Step 1: 运行报告页、时间线、摘要相关测试**
- [ ] **Step 2: 检查是否有回归并修正**
