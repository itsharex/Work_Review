# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.3] - 2026-03-12

### 新增
- 智能空闲检测：自动解决"应用挂着不用但时长继续累加"的问题
  - 两阶段检测机制：
    - 第一阶段：键鼠活动检测（Windows: `GetLastInputInfo`，macOS: `CGEventSourceSecondsSinceLastEventType`）
    - 第二阶段：屏幕内容变化检测（截图哈希比对）
  - 智能判断逻辑：
    - 有键鼠操作 → 活跃，正常计时
    - 无键鼠操作但屏幕有变化 → 活跃（终端运行程序、视频播放等场景）
    - 无键鼠操作且屏幕连续 3 次无变化 → 空闲，暂停计时
  - 跨平台支持：Windows 和 macOS 双平台完整实现
  - 无需配置：固定 3 分钟阈值，自动工作，用户无感知

### 修复
- 修复 Windows 10 截图和应用统计不工作的问题
  - 新增 GDI BitBlt 截图备用方案，当 Windows Graphics Capture API 失败时自动降级
  - Windows 11 优先使用 WGC API（高性能），Windows 10 自动切换到 GDI 方案（兼容性优先）
  - 解决了 Win10 与 Win11 截图 API 不兼容导致的记录缺失问题

## [1.0.2] - 2026-03-11

### 修复
- 修复 Windows 10 不被记录的问题
  - 改用 `PROCESS_QUERY_LIMITED_INFORMATION` 替代高权限标志，覆盖 UAC 保护进程和 Microsoft Store 应用
  - 增加 `QueryFullProcessImageNameW` 和窗口标题推断两级备用方案，基本消除 Unknown 进程记录
  - 新增 `winbase` feature 支持
- 修复活动总时长严重偏低的问题（Unknown 进程时长全部堆积到同一条记录）

### 优化
- 概览页布局优化：网站访问和应用使用区块始终渲染，消除加载时的 layout shift
- 关于页删除无意义的"数据安全保障"说明卡片
- 侧边栏 Logo 区域视觉优化（图标和间距收紧）

## [1.0.1] - 2026-03-11


### 修复
- 修复 Windows 10 启动后概况数据全为 0 的问题
  - 锁屏检测：使用 OpenInputDesktop 替代不可靠的 GetForegroundWindow/quser 判断
  - 截屏兼容：DrawBorderSettings::WithoutBorder 在 Win10 不支持时自动降级到 WithBorder
  - 窗口获取：GetForegroundWindow 返回 null 时降级为 Desktop，避免跳过轮询

### 优化
- 修复 Rust 后端编译警告
- 修复前端 A11y 可访问性警告
- Clippy 代码风格优化

## [1.0.0] 

### 核心功能
- 智能工作追踪：自动记录应用使用情况
- 活动合并：同一应用连续使用自动合并为单条记录
- 浏览器 URL 追踪：记录访问的网站
- 时间线视图：可视化查看每日工作轨迹
- AI 日报生成：支持本地 Ollama 和云端 API

### 隐私保护
- 应用规则：跳过、模糊或正常记录指定应用
- 敏感关键词过滤：自动过滤包含敏感词的活动
- 域名黑名单：不记录指定网站
- 锁屏检测：锁屏时自动暂停记录

### 平台支持
- macOS 10.13+：屏幕录制权限检测
- Windows 10+：完整功能支持

### 技术规格
- 后端：Rust + Tauri 2
- 前端：Svelte 4 + TailwindCSS
- 数据库：SQLite
