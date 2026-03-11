// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// macOS: objc 宏（msg_send!, class! 等）需要 macro_use 全局导入
#[cfg(target_os = "macos")]
#[macro_use]
extern crate objc;

mod analysis;
mod commands;
mod config;
mod database;
mod error;
mod monitor;
mod ocr;
mod ocr_logger;
mod privacy;
mod screen_lock;
mod screenshot;
mod storage;

use chrono::Local;
use config::AppConfig;
use database::Database;
use once_cell::sync::OnceCell;
use privacy::PrivacyFilter;
use screenshot::ScreenshotService;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use storage::StorageManager;
use tauri::{AppHandle, Manager, Emitter};
use tauri::menu::{MenuBuilder, MenuItemBuilder};
use tauri::tray::{TrayIconBuilder, TrayIconEvent, MouseButton, MouseButtonState};

// 全局 AppHandle，用于在 macOS Dock 点击时恢复窗口
static APP_HANDLE: OnceCell<AppHandle> = OnceCell::new();

/// 应用状态
pub struct AppState {
    pub config: AppConfig,
    pub database: Database,
    pub privacy_filter: PrivacyFilter,
    pub screenshot_service: ScreenshotService,
    pub storage_manager: StorageManager,
    pub data_dir: PathBuf,
    pub is_recording: bool,
    pub is_paused: bool,
}

/// 获取数据目录
fn get_data_dir() -> PathBuf {
    // 优先使用可执行文件所在目录
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let data_dir = exe_dir.join("data");
            if std::fs::create_dir_all(&data_dir).is_ok() {
                return data_dir;
            }
        }
    }

    // 后备：使用用户数据目录
    dirs::data_dir()
        .map(|d| d.join("work-review"))
        .unwrap_or_else(|| PathBuf::from("./data"))
}

// 系统托盘在 setup 钩子中使用 TrayIconBuilder 创建 (Tauri v2)

/// 后台截屏任务
/// 使用 Arc<Mutex<AppState>> 而非 tauri::State，因为 State 无法在 async move 块中手动构造
async fn background_screenshot_task(state: Arc<Mutex<AppState>>, window: tauri::WebviewWindow) {
    // ===== 精确时长计算变量 =====
    // 初始化变量
    let _last_capture_time = std::time::Instant::now();
    let mut last_app_change_time = std::time::Instant::now();
    let mut _last_app_change_wall_time = Local::now(); // 记录墙钟时间用于跨天检测（预留）
    let mut last_app_name: Option<String> = None;
    let mut _last_app_window_title: Option<String> = None;
    let mut _last_browser_url: Option<String> = None; // 预留

    let mut last_capture_time = std::time::Instant::now();

    const MIN_CAPTURE_INTERVAL_MS: u128 = 3000; // 最小截图间隔3秒（防抖）
    const POLL_INTERVAL_SECS: u64 = 5; // 轮询间隔5秒（更精确的时长计算）

    loop {
        // 检测屏幕锁定状态，锁屏时不统计时长
        let screen_lock_monitor = screen_lock::ScreenLockMonitor::new();
        if screen_lock_monitor.is_locked() {
            log::info!("🔒 屏幕已锁定，暂停活动统计");
            // 重置计时基准，避免解锁后累加锁屏期间的时长
            last_app_change_time = std::time::Instant::now();
            _last_app_change_wall_time = Local::now();
            last_app_name = None; // 重置应用状态，解锁后视为新开始
            tokio::time::sleep(Duration::from_secs(5)).await;
            continue;
        }

        // 首先检查录制状态并获取配置
        let (should_continue, screenshot_interval) = {
            let state_guard = state.lock().unwrap();
            if !state_guard.is_recording || state_guard.is_paused {
                (false, 1u64)
            } else {
                (true, state_guard.config.screenshot_interval)
            }
        };

        if !should_continue {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // 轮询检测活动窗口（5秒间隔，平衡精确性和性能）
        tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;

        // 获取当前活动窗口
        let active_window = match monitor::get_active_window() {
            Ok(w) => w,
            Err(e) => {
                log::error!("获取活动窗口失败: {e}");
                continue;
            }
        };

        // 再次检查状态
        let should_capture = {
            let state_guard = state.lock().unwrap();
            state_guard.is_recording && !state_guard.is_paused
        };

        if !should_capture {
            continue;
        }

        // ===== 检测应用切换 =====
        let app_changed = match &last_app_name {
            Some(last) => last != &active_window.app_name,
            None => true,
        };

        // 计算距离上次截图的时间
        let elapsed_since_capture = last_capture_time.elapsed();
        let elapsed_secs = elapsed_since_capture.as_secs();

        // ===== 精确时长计算 =====
        // duration 由每次截图/合并时的 POLL_INTERVAL_SECS 增量统一计时
        // 这里只做应用切换检测和状态重置，不再通过 add_duration 重复累加
        let _actual_duration = if app_changed && last_app_name.is_some() {
            let now_wall_time = Local::now();
            let duration = last_app_change_time.elapsed().as_secs() as i64;

            log::info!(
                "📊 应用切换: {} → {} (持续 {}秒)",
                last_app_name.as_deref().unwrap_or("无"),
                &active_window.app_name,
                duration
            );

            // 重置计时基准
            last_app_change_time = std::time::Instant::now();
            _last_app_change_wall_time = now_wall_time;
            duration
        } else {
            // 未切换，使用轮询间隔作为增量
            POLL_INTERVAL_SECS as i64
        };

        // ===== 判断是否截图 =====
        // 1. 定时触发：到达配置的间隔时间
        // 2. 应用切换触发：满足最小间隔
        let should_take_screenshot = if elapsed_secs >= screenshot_interval {
            log::debug!("定时截图触发");
            true
        } else if app_changed && elapsed_since_capture.as_millis() >= MIN_CAPTURE_INTERVAL_MS {
            log::debug!("应用切换截图触发");
            true
        } else {
            false
        };

        // 更新上一个应用的信息（无论是否截图）
        last_app_name = Some(active_window.app_name.clone());
        _last_app_window_title = Some(active_window.window_title.clone());
        _last_browser_url = active_window.browser_url.clone();

        if !should_take_screenshot {
            continue;
        }

        // 更新截图时间
        last_capture_time = std::time::Instant::now();

        // 使用距离上次截图的实际经过时间作为本次记录的时长
        // 而非固定的 POLL_INTERVAL_SECS，避免截图间隔大于轮询间隔时丢失时长
        let (privacy_action, duration_to_record) = {
            let state_guard = state.lock().unwrap();
            let action = state_guard
                .privacy_filter
                .check_privacy(&active_window.app_name, &active_window.window_title);
            // elapsed_secs 是距离上次截图的真实秒数，确保时长不丢失
            let duration = elapsed_secs.max(POLL_INTERVAL_SECS) as i64;
            (action, duration)
        };
        // 锁已释放

        use privacy::PrivacyAction;
        let result: Option<database::Activity> = match privacy_action {
            PrivacyAction::Skip => {
                log::debug!(
                    "完全跳过: {} - {}",
                    active_window.app_name,
                    active_window.window_title
                );
                None
            }
            PrivacyAction::Anonymize => {
                log::debug!(
                    "内容脱敏: {} - {}",
                    active_window.app_name,
                    active_window.window_title
                );
                let category = monitor::categorize_app(&active_window.app_name);
                let activity = database::Activity {
                    id: None,
                    timestamp: chrono::Local::now().timestamp(),
                    app_name: active_window.app_name,
                    window_title: "[内容已脱敏]".to_string(),
                    screenshot_path: String::new(),
                    ocr_text: None,
                    category,
                    duration: duration_to_record,
                    browser_url: None,
                };

                // 短暂获取锁写入数据库
                let state_guard = state.lock().unwrap();
                match state_guard.database.insert_activity(&activity) {
                    Ok(_) => Some(activity),
                    Err(e) => {
                        log::error!("保存活动记录失败: {e}");
                        None
                    }
                }
            }
            PrivacyAction::Record => {
                let category = monitor::categorize_app(&active_window.app_name);
                let current_timestamp = chrono::Local::now().timestamp();

                // 先检查是否有可合并的记录（在截屏之前判断，避免不必要的截图保存）
                let latest_activity = {
                    let state_guard = state.lock().unwrap();
                    if let Some(ref url) = active_window.browser_url {
                        if !url.is_empty() {
                            state_guard
                                .database
                                .get_latest_activity_by_url(url)
                                .ok()
                                .flatten()
                        } else {
                            state_guard
                                .database
                                .get_latest_activity_by_app(&active_window.app_name)
                                .ok()
                                .flatten()
                        }
                    } else {
                        state_guard
                            .database
                            .get_latest_activity_by_app(&active_window.app_name)
                            .ok()
                            .flatten()
                    }
                };

                let is_merge = latest_activity.is_some();

                if is_merge {
                    // === 合并路径：不保存截图，只做 OCR ===
                    let latest = latest_activity.unwrap();
                    let latest_id = latest.id.unwrap();

                    // 截屏到内存，保存为临时文件供 OCR 使用
                    let screenshot_result = {
                        let state_guard = state.lock().unwrap();
                        state_guard.screenshot_service.capture()
                    };

                    // 合并记录（不更新 screenshot_path，保留活动创建时的原始截图）
                    {
                        let state_guard = state.lock().unwrap();
                        match state_guard.database.merge_activity(
                            latest_id,
                            duration_to_record,
                            None,
                            &latest.screenshot_path, // 保留原始截图路径不变
                            current_timestamp,
                        ) {
                            Ok(_) => {
                                log::info!(
                                    "✅ 合并成功: {} (id={}, 新时长={}s)",
                                    active_window.app_name,
                                    latest_id,
                                    latest.duration + duration_to_record
                                );
                            }
                            Err(e) => {
                                log::error!("合并活动记录失败: {e}");
                            }
                        }
                    }

                    // 对截图执行 OCR，然后删除临时截图
                    if let Ok(screenshot) = screenshot_result {
                        let temp_path = screenshot.path.clone();
                        let state_clone = state.clone();
                        let data_dir_clone = {
                            let state_guard = state.lock().unwrap();
                            state_guard.data_dir.clone()
                        };

                        use std::sync::atomic::{AtomicU64, Ordering};
                        static MERGE_SCREENSHOT_HASH: AtomicU64 = AtomicU64::new(0);

                        tokio::spawn(async move {
                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

                            // 计算哈希做去重判断
                            let current_hash =
                                screenshot::ScreenshotService::calculate_image_hash(&temp_path)
                                    .unwrap_or(0);
                            let last_hash =
                                MERGE_SCREENSHOT_HASH.swap(current_hash, Ordering::Relaxed);

                            let should_ocr = if last_hash != 0 {
                                let similarity = screenshot::ScreenshotService::hash_similarity(
                                    last_hash,
                                    current_hash,
                                );
                                if similarity > 90 {
                                    log::debug!("合并截图相似度 {similarity}%，跳过 OCR");
                                    false
                                } else {
                                    log::debug!("合并截图相似度 {similarity}%，执行 OCR");
                                    true
                                }
                            } else {
                                true
                            };

                            if should_ocr {
                                let ocr_service = ocr::OcrService::new(&data_dir_clone);
                                if let Ok(Some(ocr_result)) =
                                    ocr_service.extract_text(&temp_path)
                                {
                                    if !ocr_result.text.is_empty() {
                                        let filtered_text =
                                            ocr::filter_sensitive_text(&ocr_result.text);
                                        if let Ok(state_guard) = state_clone.lock() {
                                            let _ = state_guard.database.update_activity_ocr(
                                                latest_id,
                                                Some(filtered_text),
                                            );
                                            log::info!(
                                                "OCR 完成(合并): 活动 {} 识别到 {} 个字符",
                                                latest_id,
                                                ocr_result.text.len()
                                            );
                                        }
                                    }
                                }
                            }

                            // 删除临时截图文件（不保留）
                            let _ = std::fs::remove_file(&temp_path);
                            log::debug!("已删除临时截图: {temp_path:?}");
                        });
                    }

                    Some(database::Activity {
                        id: Some(latest_id),
                        timestamp: current_timestamp,
                        app_name: active_window.app_name.clone(),
                        window_title: active_window.window_title,
                        screenshot_path: latest.screenshot_path,
                        ocr_text: None,
                        category,
                        duration: latest.duration + duration_to_record,
                        browser_url: active_window.browser_url,
                    })
                } else {
                    // === 新建路径：正常截屏并保存 ===
                    let screenshot_result = {
                        let state_guard = state.lock().unwrap();
                        state_guard.screenshot_service.capture()
                    };

                    match screenshot_result {
                        Ok(screenshot_result) => {
                            let (relative_path, data_dir_clone) = {
                                let state_guard = state.lock().unwrap();
                                (
                                    state_guard
                                        .screenshot_service
                                        .get_relative_path(&screenshot_result.path),
                                    state_guard.data_dir.clone(),
                                )
                            };

                            let activity = database::Activity {
                                id: None,
                                timestamp: screenshot_result.timestamp,
                                app_name: active_window.app_name.clone(),
                                window_title: active_window.window_title,
                                screenshot_path: relative_path.clone(),
                                ocr_text: None,
                                category,
                                duration: duration_to_record,
                                browser_url: active_window.browser_url,
                            };

                            let inserted = {
                                let state_guard = state.lock().unwrap();
                                state_guard.database.insert_activity(&activity)
                            };

                            match inserted {
                                Ok(activity_id) => {
                                    log::info!(
                                        "📝 新建活动: {} (id={})",
                                        active_window.app_name,
                                        activity_id
                                    );

                                    // 异步 OCR（新建活动的截图已保存，不删除）
                                    let state_clone = state.clone();
                                    let screenshot_path_clone = relative_path;
                                    tokio::spawn(async move {
                                        tokio::time::sleep(
                                            tokio::time::Duration::from_secs(1),
                                        )
                                        .await;

                                        let full_path =
                                            data_dir_clone.join(&screenshot_path_clone);
                                        let ocr_service =
                                            ocr::OcrService::new(&data_dir_clone);

                                        if let Ok(Some(ocr_result)) =
                                            ocr_service.extract_text(&full_path)
                                        {
                                            if !ocr_result.text.is_empty() {
                                                let filtered_text =
                                                    ocr::filter_sensitive_text(&ocr_result.text);
                                                if let Ok(state_guard) = state_clone.lock() {
                                                    let _ =
                                                        state_guard.database.update_activity_ocr(
                                                            activity_id,
                                                            Some(filtered_text),
                                                        );
                                                    log::info!(
                                                        "OCR 完成(新建): 活动 {} 识别到 {} 个字符",
                                                        activity_id,
                                                        ocr_result.text.len()
                                                    );
                                                }
                                            }
                                        }
                                    });

                                    Some(database::Activity {
                                        id: Some(activity_id),
                                        ..activity
                                    })
                                }
                                Err(e) => {
                                    log::error!("保存活动记录失败: {e}");
                                    None
                                }
                            }
                        }
                        Err(e) => {
                            log::error!("截屏失败: {e}");
                            None
                        }
                    }
                }
            }
        };

        // 发送事件到前端
        if let Some(activity) = result {
            let _ = window.emit("screenshot-taken", &activity);
        }
    }
}

/// 小时摘要生成任务
/// 每小时检查一次，为上一个完整小时生成摘要
/// 为指定日期和小时生成并保存摘要
fn generate_and_save_summary(
    state: &Arc<Mutex<AppState>>,
    date: &str,
    hour: i32,
) {
    use analysis::hourly::{generate_fallback_summary, HourlyStats};

    let activities = {
        let state_guard = state.lock().unwrap();
        state_guard.database.get_hourly_activities(date, hour)
    };

    match activities {
        Ok(acts) if !acts.is_empty() => {
            let stats = HourlyStats::from_activities(date, hour, acts);
            let summary = generate_fallback_summary(&stats);

            let hourly_summary = database::HourlySummary {
                id: None,
                date: date.to_string(),
                hour,
                summary,
                main_apps: stats.get_main_apps().join(", "),
                activity_count: stats.activity_count,
                total_duration: stats.total_duration,
                representative_screenshots: Some(
                    serde_json::to_string(&stats.representative_screenshots)
                        .unwrap_or_default(),
                ),
                created_at: chrono::Local::now().timestamp(),
            };

            let save_result = {
                let state_guard = state.lock().unwrap();
                state_guard.database.save_hourly_summary(&hourly_summary)
            };

            match save_result {
                Ok(_) => log::info!("小时摘要保存成功: {date} {hour}:00"),
                Err(e) => log::error!("保存小时摘要失败: {e}"),
            }
        }
        Ok(_) => {
            log::debug!("该小时无活动数据: {date} {hour}:00");
        }
        Err(e) => {
            log::error!("获取小时活动数据失败: {e}");
        }
    }
}

async fn hourly_summary_task(state: Arc<Mutex<AppState>>) {
    use chrono::{Local, Timelike};

    // 等待30秒后开始（给应用启动留时间，但不用等太久）
    tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;

    // 启动时回填今天所有已过时段的摘要（覆盖旧格式数据）
    {
        let now = Local::now();
        let date = now.format("%Y-%m-%d").to_string();
        let current_hour = now.hour() as i32;

        log::info!("回填今天 0:00 ~ {current_hour}:00 的小时摘要");
        for hour in 0..current_hour {
            generate_and_save_summary(&state, &date, hour);
        }
    }

    loop {
        let now = Local::now();
        let current_hour = now.hour() as i32;
        let date = now.format("%Y-%m-%d").to_string();

        // 为上一个小时生成摘要（如果还没有）
        let target_hour = if current_hour > 0 {
            current_hour - 1
        } else {
            23
        };
        let target_date = if current_hour > 0 {
            date.clone()
        } else {
            (now - chrono::Duration::days(1))
                .format("%Y-%m-%d")
                .to_string()
        };

        // 检查是否已有摘要
        let should_generate = {
            let state_guard = state.lock().unwrap();
            match state_guard
                .database
                .has_hourly_summary(&target_date, target_hour)
            {
                Ok(has) => !has,
                Err(e) => {
                    log::error!("检查小时摘要失败: {e}");
                    false
                }
            }
        };

        if should_generate {
            log::info!("开始生成 {target_date} {target_hour}:00 的小时摘要");
            generate_and_save_summary(&state, &target_date, target_hour);
        }

        // 休眠到下一个小时的第5分钟
        let next_check = (now + chrono::Duration::hours(1))
            .with_minute(5)
            .unwrap()
            .with_second(0)
            .unwrap();
        let sleep_duration = (next_check - now).num_seconds().max(60) as u64;
        tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration)).await;
    }
}

#[tauri::command]
fn get_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "unknown";
}

#[tokio::main]
async fn main() {
    // 初始化日志
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    log::info!("work回顾助手启动中...");

    // 获取数据目录
    let data_dir = get_data_dir();
    log::info!("数据目录: {data_dir:?}");

    // 加载配置
    let config_path = data_dir.join("config.json");
    let config = AppConfig::load(&config_path).unwrap_or_else(|e| {
        log::warn!("加载配置失败，使用默认配置: {e}");
        AppConfig::default()
    });

    // 初始化数据库
    let db_path = data_dir.join("workreview.db");
    let database = Database::new(&db_path).expect("初始化数据库失败");

    // 初始化隐私过滤器
    let privacy_filter = PrivacyFilter::from_config(&config.privacy);

    // 初始化截屏服务
    let screenshot_service = ScreenshotService::new(&data_dir);

    // macOS: 启动时检查屏幕录制权限
    // 只在首次启动时触发权限请求，避免重复弹窗
    #[cfg(target_os = "macos")]
    {
        if !screenshot::has_screen_capture_permission() {
            log::warn!("屏幕录制权限未授权，请在系统设置中授权后重启应用");
            // 触发一次截屏尝试来请求权限（系统会弹出授权对话框）
            let _ = screenshot_service.capture();
        } else {
            log::info!("屏幕录制权限已授权");
        }
    }

    // 初始化存储管理器
    let storage_manager = StorageManager::new(&data_dir, config.storage.clone());

    // 启动时执行一次清理
    if let Err(e) = storage_manager.cleanup() {
        log::warn!("启动时清理存储失败: {e}");
    }

    // 创建应用状态，使用 Arc 包装以便在多个地方共享
    let app_state = Arc::new(Mutex::new(AppState {
        config,
        database,
        privacy_filter,
        screenshot_service,
        storage_manager,
        data_dir,
        is_recording: true,
        is_paused: false,
    }));

    // 构建 Tauri 应用
    tauri::Builder::default()
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_single_instance::init(|app, argv, cwd| {
            // 当用户尝试打开第二个实例时，将焦点给到现有窗口
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
            log::info!("检测到重复打开，参数: {argv:?}, 工作目录: {cwd}");
        }))
        .manage(app_state.clone())
        // 系统托盘在 setup 中创建 (Tauri v2)
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 点击关闭按钮时隐藏窗口而不是退出
                let _ = window.hide();
                api.prevent_close();
            }
        })
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // macOS 原生标题栏配置
            #[cfg(target_os = "macos")]
            {
                use tauri::TitleBarStyle;
                // 开启 decorations 以显示红绿灯
                let _ = window.set_decorations(true);
                // 设置标题栏透明（红绿灯悬浮在内容之上）
                let _ = window.set_title_bar_style(TitleBarStyle::Transparent);
            }

            // 获取 Arc<Mutex<AppState>> 并克隆以便在异步任务中使用
            let state = app.state::<Arc<Mutex<AppState>>>();
            let state_clone = state.inner().clone();
            let state_clone2 = state.inner().clone();
            let _state_for_tray = state.inner().clone(); // 预留
            let window_clone = window.clone();
            let window_for_tray = window.clone();
            
            // 创建 Tauri v2 系统托盘
            let show = MenuItemBuilder::with_id("show", "显示窗口").build(app)?;
            let pause = MenuItemBuilder::with_id("pause", "暂停录制").build(app)?;
            let resume = MenuItemBuilder::with_id("resume", "恢复录制").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "退出").build(app)?;
            
            let menu = MenuBuilder::new(app)
                .item(&show)
                .separator()
                .item(&pause)
                .item(&resume)
                .separator()
                .item(&quit)
                .build()?;
            
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| {
                    match event.id().as_ref() {
                        "quit" => {
                            std::process::exit(0);
                        }
                        "show" => {
                            if let Some(window) = app.get_webview_window("main") {
                                let _ = window.unminimize();
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        }
                        "pause" => {
                            if let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() {
                                if let Ok(mut state) = state.lock() {
                                    state.is_paused = true;
                                    log::info!("录制已暂停");
                                }
                            }
                        }
                        "resume" => {
                            if let Some(state) = app.try_state::<Arc<Mutex<AppState>>>() {
                                if let Ok(mut state) = state.lock() {
                                    state.is_paused = false;
                                    log::info!("录制已恢复");
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(move |_tray, event| {
                    // 处理托盘图标点击
                    if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                        let _ = window_for_tray.unminimize();
                        let _ = window_for_tray.show();
                        let _ = window_for_tray.set_focus();
                    }
                })
                .build(app)?;

            // 启动后台截屏任务
            tauri::async_runtime::spawn(async move {
                background_screenshot_task(state_clone, window_clone).await;
            });

            // 启动小时摘要生成任务（每小时检查一次）
            tauri::async_runtime::spawn(async move {
                hourly_summary_task(state_clone2).await;
            });

            // 启动时清理当天的重复记录
            {
                let state_guard = state.inner().lock().unwrap();
                let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                match state_guard.database.cleanup_duplicate_activities(&today) {
                    Ok((deleted, paths)) => {
                        if deleted > 0 {
                            log::warn!("🧹 启动清理: 删除 {deleted} 条重复记录");
                            // 删除对应的截图文件
                            for p in paths {
                                let path = state_guard.data_dir.join(&p);
                                if path.exists() {
                                    let _ = std::fs::remove_file(&path);
                                }
                            }
                        }
                    }
                    Err(e) => log::error!("清理重复记录失败: {e}"),
                }

                // 启动时应用 Dock 图标配置
                #[cfg(target_os = "macos")]
                if state_guard.config.hide_dock_icon {
                    use cocoa::appkit::{
                        NSApp, NSApplication,
                        NSApplicationActivationPolicy::NSApplicationActivationPolicyAccessory,
                    };
                    unsafe {
                        NSApp().setActivationPolicy_(NSApplicationActivationPolicyAccessory);
                    }
                }

                // decorations 配置由 tauri.conf.json 控制，用户可通过设置中的开关动态修改
            }

            // 保存 AppHandle 到全局变量，用于从 macOS Dock 点击恢复窗口
            let _ = APP_HANDLE.set(app.handle().clone());

            // 注: macOS Dock 点击恢复窗口通过系统托盘 LeftClick 事件处理
            // 用户需要点击状态栏的系统托盘图标来恢复窗口

            log::info!("应用初始化完成");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_today_stats,
            commands::get_daily_stats,
            commands::get_timeline,
            commands::generate_report,
            commands::get_saved_report,
            commands::get_config,
            commands::save_config,
            commands::start_recording,
            commands::stop_recording,
            commands::pause_recording,
            commands::resume_recording,
            commands::get_recording_state,
            commands::get_data_dir,
            commands::open_data_dir,
            commands::get_screenshot_thumbnail,
            commands::get_screenshot_full,
            commands::take_screenshot,
            commands::test_ai_model,
            commands::test_model,
            commands::get_ai_providers,
            commands::get_running_apps,
            commands::get_recent_apps,
            commands::get_storage_stats,
            commands::get_hourly_summaries,
            commands::get_activity,
            commands::clear_old_activities,
            commands::get_ocr_log,
            commands::is_screen_locked,
            commands::is_work_time,
            commands::check_ocr_available,
            commands::run_ocr,
            commands::get_ocr_install_guide,
            commands::set_dock_visibility,
            commands::get_app_icon,
            get_platform,
        ])
        .build(tauri::generate_context!())
        .expect("构建 Tauri 应用时出错")
        .run(|app_handle, event| match event {
            // 处理 macOS Dock 点击：显示隐藏的窗口（仅 macOS）
            #[cfg(target_os = "macos")]
            tauri::RunEvent::Reopen { has_visible_windows, .. } => {
                if !has_visible_windows {
                    if let Some(window) = app_handle.get_webview_window("main") {
                        let _ = window.unminimize();
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            _ => {}
        });
}
