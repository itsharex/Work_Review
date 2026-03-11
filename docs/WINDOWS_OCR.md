# Windows 平台说明

## 平台适配

Work Review 在 Windows 上完全支持以下功能：

### 锁屏检测
- 使用 WinAPI 检测前台窗口状态
- 锁屏时自动暂停活动记录

### 浏览器 URL 获取
- 支持 Chrome、Edge、Brave、Firefox
- 使用 UI Automation API 从地址栏获取 URL

### OCR 文字识别
默认使用 Windows 内置 OCR API，也可选择安装 PaddleOCR：

```bash
pip install paddlepaddle paddleocr -i https://mirror.baidu.com/pypi/simple
```

## 构建 Windows 安装包

```bash
npm install
npm run tauri build
```

生成的安装包：`src-tauri/target/release/bundle/msi/*.msi`

**依赖**：[WiX Toolset 3.x](https://wixtoolset.org/)
