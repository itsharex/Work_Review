# GNOME Wayland 活动窗口来源设计

日期：2026-04-05

## 1. 背景

当前项目的 Linux 活动窗口检测仅支持 X11，依赖 `xdotool + xprop` 获取前台窗口信息。
在 Wayland 会话下，后端已经能够识别 `X11 / Wayland`，截图链路也已补齐，但
`monitor::get_active_window()` 在 Wayland 下仍直接返回“不支持”活动窗口追踪错误，
导致自动记录主链路无法恢复。

本设计用于补上 Wayland 的第一条活动窗口来源，但严格限制范围，避免一次性把
所有桌面环境、窗口几何、浏览器 URL 一并引入，造成实现面过大和验证复杂度失控。

## 2. 目标

本阶段目标只有一个：

- 在 `GNOME Wayland` 环境下恢复活动窗口的基础信息采集

本阶段返回的数据仅包含：

- `app_name`
- `window_title`

## 3. 非目标

本阶段明确不做以下内容：

- 不支持 KDE Wayland
- 不支持 Sway / Hyprland / wlroots 系桌面
- 不补 `window_bounds`
- 不补 `browser_url`
- 不尝试做跨桌面统一 Wayland 方案
- 不修改 X11 现有实现

## 4. 范围约束

### 4.1 支持矩阵

- X11：继续走现有 `xdotool + xprop`
- GNOME Wayland：新增活动窗口 provider
- 其他 Wayland：继续返回明确降级错误

### 4.2 兼容性承诺

这次改动只承诺：

- `GNOME Wayland` 下，主录制循环能够重新获取当前前台应用名与窗口标题

不承诺：

- 所有 GNOME 版本都完全一致
- 浏览器页面 URL 恢复
- 多屏选屏恢复
- 所有 Wayland 发行版通用

## 5. 设计概览

### 5.1 总体思路

在 Linux 监控入口中，先判断当前会话类型：

1. 若为 `X11`，维持原实现
2. 若为 `Wayland`，再判断桌面环境
3. 若桌面环境是 `GNOME`，走新的 GNOME Wayland provider
4. 若不是 `GNOME`，返回当前降级错误

### 5.2 推荐实现

优先采用 `gdbus` 调用 `org.gnome.Shell` 可访问接口，读取当前焦点窗口的基础元数据。

原因：

- 这是当前最贴近“GNOME Wayland 前台窗口”问题的实现
- 不需要在本阶段引入新的 Rust D-Bus 依赖
- 与现有 Linux 实现保持一致，继续通过外部命令调用，便于快速落地与回退
- 后续如果补 KDE/Sway，可以继续沿用 provider 分发结构，而不是把 GNOME 特例写死在 `get_active_window()` 主函数里

## 6. 模块设计

### 6.1 `linux_session.rs`

新增桌面环境识别能力：

- 增加 Linux 桌面环境枚举，例如：
  - `Gnome`
  - `Kde`
  - `Sway`
  - `Unknown`
- 增加当前桌面环境检测函数

检测信号优先级：

1. `XDG_CURRENT_DESKTOP`
2. `DESKTOP_SESSION`
3. 其他可用环境变量的兜底匹配

本阶段只需要可靠识别 `GNOME`，其他环境识别可以粗粒度实现。

### 6.2 `monitor.rs`

Linux 路径按 provider 分发：

- `get_active_window_linux_x11()`
- `get_active_window_linux_wayland_gnome()`

主入口逻辑：

1. 若 session 为 `X11`，调用 `get_active_window_linux_x11()`
2. 若 session 为 `Wayland` 且 desktop 为 `GNOME`，调用 `get_active_window_linux_wayland_gnome()`
3. 否则返回明确错误

### 6.3 GNOME Wayland Provider

新增一个专门的 provider 函数：

- `get_active_window_linux_wayland_gnome()`

职责：

- 调用 `gdbus`
- 获取 GNOME Shell 返回值
- 从返回文本中解析当前前台窗口的：
  - 应用名
  - 窗口标题
- 转成统一的 `ActiveWindow`

返回结构：

- `app_name`: 解析出的显示名，必要时走现有 `normalize_display_app_name`
- `window_title`: 当前窗口标题
- `browser_url`: `None`
- `executable_path`: `None`
- `window_bounds`: `None`

## 7. 数据流

```text
monitor::get_active_window()
  -> current_linux_desktop_session()
  -> current_linux_desktop_environment()
  -> GNOME Wayland provider
  -> gdbus 调用 GNOME Shell
  -> 解析 app_name + window_title
  -> 返回 ActiveWindow
  -> main.rs 录制主循环复用现有逻辑
```

## 8. GNOME Wayland Provider 解析策略

### 8.1 输入

通过 `gdbus` 调用 GNOME Shell，执行一段最小化脚本，返回当前焦点窗口的基础字段。

脚本目标：

- 定位当前焦点窗口
- 获取应用显示名
- 获取窗口标题
- 拼成可解析的简单文本结果

### 8.2 输出格式

为降低解析脆弱度，provider 侧应尽量要求返回固定结构文本，例如：

```text
app_name=Firefox
window_title=OpenAI Docs - Mozilla Firefox
```

Rust 侧只解析这两个键，避免在本阶段引入复杂 JSON 解码或对 GNOME Shell 内部对象结构做过多假设。

### 8.3 解析原则

- 缺少 `app_name` 或 `window_title` 时视为失败
- 对空白、引号和换行做清理
- `app_name` 进入现有归一化逻辑
- 不从标题中反推 URL

## 9. 错误处理与降级

### 9.1 明确失败，不伪造数据

以下情况都应返回错误，而不是构造假活动窗口：

- `gdbus` 不存在
- GNOME Shell 接口不可访问
- 返回内容为空
- 返回中缺少 `app_name` 或 `window_title`
- 当前桌面不是 `GNOME`

### 9.2 错误文案原则

错误信息要能区分：

- `Wayland + 非 GNOME`：当前阶段未适配
- `GNOME Wayland + gdbus 不可用`：依赖缺失
- `GNOME Wayland + 解析失败`：provider 结果异常

这样后续关于页和日志才能给出足够明确的反馈。

## 10. 对现有链路的影响

### 10.1 录制主循环

`main.rs` 主循环无需改动业务流程。只要 `monitor::get_active_window()` 能返回
有效 `ActiveWindow`，现有录制、分类、OCR、时间线等逻辑都能继续复用。

### 10.2 浏览器 URL

本阶段 `browser_url = None`，所以：

- 浏览器页面级追踪在 GNOME Wayland 下仍不会恢复
- 但浏览器应用级记录会恢复

### 10.3 多屏截图

由于 `window_bounds = None`：

- GNOME Wayland 下截图仍不会恢复“按活动窗口所在屏幕选屏”
- 这属于后续阶段

## 11. 测试设计

先写失败测试，再补实现。

### 11.1 静态测试

新增或扩展 Node 侧源码测试，覆盖：

- `linux_session.rs` 新增桌面环境枚举与检测
- `monitor.rs` 存在 `GNOME Wayland` 分流
- `monitor.rs` 存在 `get_active_window_linux_wayland_gnome`
- `gdbus` 调用存在
- `GNOME Wayland` provider 返回 `app_name` 与 `window_title`
- 非 GNOME Wayland 仍保留降级路径

### 11.2 Rust 模块测试

优先补纯函数测试，不依赖真实桌面环境：

- 桌面环境识别函数测试
- `gdbus` 输出解析函数测试
- 解析失败输入测试
- 正常输出映射到 `ActiveWindow` 的测试

## 12. 风险

### 12.1 GNOME 接口脆弱

GNOME Shell 的可访问接口在不同版本上可能有差异。

控制方式：

- provider 独立封装
- 文本协议尽量最小化
- 失败时明确报错，不污染主逻辑

### 12.2 外部命令依赖

`gdbus` 可能并非所有系统都默认存在。

控制方式：

- 启动失败时返回清晰错误
- 不把 provider 成功作为 Linux Wayland 的全局默认假设

### 12.3 仅恢复基础信息

用户可能误以为 Wayland 已“完全支持”。

控制方式：

- 关于页文案更新为“GNOME Wayland 恢复基础窗口追踪”
- 保留对浏览器 URL / window bounds 的未适配提示

## 13. 后续阶段

如果本阶段稳定，下一步按顺序推进：

1. GNOME Wayland 补 `window_bounds`
2. GNOME Wayland 补浏览器 URL 的可行性调研
3. KDE Wayland provider
4. wlroots / Sway provider

## 14. 实施检查清单

- 新增 Linux 桌面环境识别
- 为 Linux 活动窗口检测补 provider 分发结构
- 实现 GNOME Wayland provider
- 仅返回 `app_name + window_title`
- 其他 Wayland 桌面继续明确降级
- 补充静态测试与 Rust 单元测试
- 更新关于页兼容性说明
