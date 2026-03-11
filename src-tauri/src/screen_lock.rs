// 锁屏检测模块 (Windows / macOS)
// 监听系统锁屏/解锁事件，用于控制录制状态

#![allow(dead_code)]

use chrono::Timelike;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 屏幕锁定状态
pub struct ScreenLockMonitor {
    /// 是否锁定
    is_locked: Arc<AtomicBool>,
}

impl ScreenLockMonitor {
    /// 创建锁屏监控器
    pub fn new() -> Self {
        Self {
            is_locked: Arc::new(AtomicBool::new(false)),
        }
    }

    /// 检查屏幕是否锁定 (Windows)
    #[cfg(target_os = "windows")]
    pub fn is_locked(&self) -> bool {
        use std::ffi::OsString;
        use std::os::windows::ffi::OsStringExt;
        use winapi::um::winuser::{GetForegroundWindow, GetWindowTextW};

        unsafe {
            // 方法1: 检查前台窗口是否为锁屏窗口
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                // 没有前台窗口，可能处于锁屏状态
                return true;
            }

            // 获取窗口标题
            let mut title: [u16; 256] = [0; 256];
            let len = GetWindowTextW(hwnd, title.as_mut_ptr(), 256);
            if len > 0 {
                let window_title = OsString::from_wide(&title[..len as usize])
                    .to_string_lossy()
                    .to_lowercase();

                // 检查是否为锁屏相关窗口
                if window_title.contains("windows 登录")
                    || window_title.contains("windows security")
                    || window_title.contains("lock screen")
                    || window_title.contains("锁屏")
                {
                    return true;
                }
            }
        }

        // 方法2: 检查工作站锁定状态
        {
            use std::os::windows::process::CommandExt;
            use std::process::Command;

            // CREATE_NO_WINDOW 标志，防止弹出黑色控制台窗口
            const CREATE_NO_WINDOW: u32 = 0x08000000;

            // 使用 PowerShell 检查会话锁定状态
            let output = Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    r#"
                    $query = quser 2>$null
                    if ($query -match 'Disc') { 
                        'locked' 
                    } else { 
                        'unlocked' 
                    }
                    "#,
                ])
                .creation_flags(CREATE_NO_WINDOW)
                .output();

            if let Ok(out) = output {
                let stdout = String::from_utf8_lossy(&out.stdout);
                if stdout.trim() == "locked" {
                    return true;
                }
            }
        }

        false
    }

    /// 检查屏幕是否锁定 (macOS)
    /// 使用多种方法检测，避免依赖 Python/pyobjc
    #[cfg(target_os = "macos")]
    pub fn is_locked(&self) -> bool {
        use std::process::Command;

        // 方法1: 检查是否有屏幕保护程序运行
        let output = Command::new("pgrep")
            .args(["-x", "ScreenSaverEngine"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                log::debug!("锁屏检测: 屏幕保护程序运行中");
                return true;
            }
        }

        // 方法2: 使用 ioreg 检测显示器电源状态
        // 当屏幕关闭（锁屏后自动关闭）时，IODisplayWrangler 的 DevicePowerState 为 0
        let output = Command::new("ioreg")
            .args(["-r", "-c", "IODisplayWrangler", "-d", "1"])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            // 检查 DevicePowerState = 0 表示显示器已关闭
            if stdout.contains("\"DevicePowerState\" = 0") {
                log::debug!("锁屏检测: 显示器已关闭");
                return true;
            }
        }

        // 方法3: 使用 osascript 检查屏幕保护状态
        let output = Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to return running of screen saver preferences",
            ])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            if stdout.trim() == "true" {
                log::debug!("锁屏检测: 屏幕保护已激活");
                return true;
            }
        }

        // 方法4: 检查 loginwindow 进程是否在前台（用户在锁屏界面）
        let output = Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to get name of first application process whose frontmost is true"])
            .output();

        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let frontmost = stdout.trim().to_lowercase();
            if frontmost == "loginwindow" || frontmost == "screensaverengine" {
                log::debug!("锁屏检测: 前台应用为锁屏界面");
                return true;
            }
        }

        false
    }

    /// 检查屏幕是否锁定 (其他平台)
    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    pub fn is_locked(&self) -> bool {
        false
    }

    /// 设置锁定状态（用于手动更新）
    pub fn set_locked(&self, locked: bool) {
        self.is_locked.store(locked, Ordering::SeqCst);
    }

    /// 检查是否在工作时间内
    pub fn is_work_time(start_hour: u8, end_hour: u8) -> bool {
        let now = chrono::Local::now();
        let hour = now.hour() as u8;

        if start_hour <= end_hour {
            // 正常时间范围，如 9-18
            hour >= start_hour && hour < end_hour
        } else {
            // 跨午夜，如 22-6
            hour >= start_hour || hour < end_hour
        }
    }
}

impl Default for ScreenLockMonitor {
    fn default() -> Self {
        Self::new()
    }
}
