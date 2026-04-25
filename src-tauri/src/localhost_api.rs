use crate::commands;
use crate::config::DEFAULT_LOCALHOST_API_PORT;
use crate::error::{AppError, Result};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::net::TcpListener as StdTcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::AppHandle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use uuid::Uuid;

pub const LOCALHOST_API_HOST: &str = "127.0.0.1";
const LOCALHOST_API_TOKEN_FILE: &str = "localhost_api_token.txt";
const MOCK_CONTROL_PLANE_STORE_FILE: &str = "node_control_plane_store.json";
const MAX_REQUEST_BYTES: usize = 256 * 1024;
const MAX_BODY_BYTES: usize = 128 * 1024;
const MOCK_NODE_HEARTBEAT_INTERVAL_SECS: u64 = 300;

pub struct LocalhostApiRuntime {
    pub running: bool,
    pub bound_port: Option<u16>,
    pub last_error: Option<String>,
    pub shutdown_tx: Option<oneshot::Sender<()>>,
    control_plane_store: MockControlPlaneStore,
}

impl Default for LocalhostApiRuntime {
    fn default() -> Self {
        Self {
            running: false,
            bound_port: None,
            last_error: None,
            shutdown_tx: None,
            control_plane_store: MockControlPlaneStore::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LocalhostApiStatusPayload {
    pub enabled: bool,
    pub running: bool,
    pub host: String,
    pub port: u16,
    pub base_url: String,
    pub token_configured: bool,
    pub token_preview: Option<String>,
    pub last_error: Option<String>,
    pub recording: bool,
    pub paused: bool,
    pub app_version: String,
    pub platform: String,
    pub device_id: String,
    pub device_name: String,
    pub node_protocol_version: String,
    pub control_plane_enabled: bool,
    pub control_plane_endpoint: Option<String>,
    pub control_plane_configured: bool,
    pub registration_state: String,
}

#[derive(Debug, Deserialize)]
struct GenerateReportRequest {
    date: String,
    #[serde(default)]
    force: Option<bool>,
    #[serde(default)]
    locale: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExportReportRequest {
    date: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    export_dir: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeRegisterRequest {
    protocol_version: String,
    device_id: String,
    device_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeHeartbeatRequest {
    protocol_version: String,
    installation_id: String,
    device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct NodeInstallationRecord {
    installation_id: String,
    device_id: String,
    device_name: String,
    device_token: String,
    last_heartbeat_at: Option<i64>,
}

#[derive(Debug, Clone)]
struct NodeRegisterResult {
    installation_id: String,
    device_token: String,
    heartbeat_interval_secs: Option<u64>,
}

#[derive(Debug)]
enum ControlPlaneStoreError {
    Unauthorized(String),
    BadRequest(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequestAuthMode {
    None,
    DeviceToken,
    LocalApiToken,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MockControlPlaneStore {
    #[serde(default)]
    installations: HashMap<String, NodeInstallationRecord>,
}

impl MockControlPlaneStore {
    fn register_or_reuse_device(
        &mut self,
        device_id: &str,
        device_name: &str,
    ) -> NodeRegisterResult {
        if let Some(existing) = self
            .installations
            .values_mut()
            .find(|record| record.device_id == device_id)
        {
            existing.device_name = device_name.to_string();
            return NodeRegisterResult {
                installation_id: existing.installation_id.clone(),
                device_token: existing.device_token.clone(),
                heartbeat_interval_secs: Some(MOCK_NODE_HEARTBEAT_INTERVAL_SECS),
            };
        }

        let installation_id = format!("inst_{}", Uuid::new_v4().simple());
        let device_token = format!("wr-node-{}", Uuid::new_v4().simple());

        self.installations.insert(
            installation_id.clone(),
            NodeInstallationRecord {
                installation_id: installation_id.clone(),
                device_id: device_id.to_string(),
                device_name: device_name.to_string(),
                device_token: device_token.clone(),
                last_heartbeat_at: None,
            },
        );

        NodeRegisterResult {
            installation_id,
            device_token,
            heartbeat_interval_secs: Some(MOCK_NODE_HEARTBEAT_INTERVAL_SECS),
        }
    }

    fn validate_and_record_heartbeat(
        &mut self,
        token: &str,
        installation_id: &str,
        device_id: &str,
    ) -> std::result::Result<Option<u64>, ControlPlaneStoreError> {
        let Some(record) = self
            .installations
            .values_mut()
            .find(|record| record.device_token == token)
        else {
            return Err(ControlPlaneStoreError::Unauthorized(
                "设备 token 无效，请重新注册".to_string(),
            ));
        };

        if record.installation_id != installation_id {
            return Err(ControlPlaneStoreError::BadRequest(
                "installation_id 与设备 token 不匹配".to_string(),
            ));
        }

        if record.device_id != device_id {
            return Err(ControlPlaneStoreError::BadRequest(
                "device_id 与设备 token 不匹配".to_string(),
            ));
        }

        record.last_heartbeat_at = Some(chrono::Utc::now().timestamp());
        Ok(Some(MOCK_NODE_HEARTBEAT_INTERVAL_SECS))
    }
}

#[derive(Debug)]
struct ParsedRequest {
    method: String,
    path: String,
    query: HashMap<String, String>,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

struct HttpResponse {
    status: u16,
    reason: &'static str,
    content_type: &'static str,
    body: Vec<u8>,
}

impl HttpResponse {
    fn json<T: Serialize>(status: u16, payload: &T) -> Self {
        let reason = reason_phrase(status);
        let body = serde_json::to_vec(payload).unwrap_or_else(|_| {
            serde_json::to_vec(&serde_json::json!({
                "error": "响应序列化失败",
            }))
            .unwrap_or_else(|_| b"{\"error\":\"serialization failed\"}".to_vec())
        });
        Self {
            status,
            reason,
            content_type: "application/json; charset=utf-8",
            body,
        }
    }

    fn error(status: u16, message: impl Into<String>) -> Self {
        Self::json(
            status,
            &serde_json::json!({
                "error": message.into(),
            }),
        )
    }

    fn text(status: u16, message: impl Into<String>) -> Self {
        Self {
            status,
            reason: reason_phrase(status),
            content_type: "text/plain; charset=utf-8",
            body: message.into().into_bytes(),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let headers = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\nCache-Control: no-store\r\n\r\n",
            self.status,
            self.reason,
            self.content_type,
            self.body.len()
        );
        let mut bytes = headers.into_bytes();
        bytes.extend_from_slice(&self.body);
        bytes
    }
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        200 => "OK",
        201 => "Created",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        405 => "Method Not Allowed",
        413 => "Payload Too Large",
        500 => "Internal Server Error",
        _ => "OK",
    }
}

fn localhost_api_token_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LOCALHOST_API_TOKEN_FILE)
}

fn mock_control_plane_store_path(data_dir: &Path) -> PathBuf {
    data_dir.join(MOCK_CONTROL_PLANE_STORE_FILE)
}

fn generate_localhost_api_token() -> String {
    format!("wr-local-{}", Uuid::new_v4().simple())
}

#[cfg(unix)]
fn open_secret_file(path: &Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;

    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true).mode(0o600);
    options.open(path)
}

#[cfg(not(unix))]
fn open_secret_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    options.open(path)
}

fn write_localhost_api_token(path: &Path, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = open_secret_file(path)?;
    file.write_all(token.as_bytes())?;
    file.flush()?;
    Ok(())
}

fn read_localhost_api_token_from_path(path: &Path) -> Result<Option<String>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)?;
    let token = content.trim().to_string();
    if token.is_empty() {
        Ok(None)
    } else {
        Ok(Some(token))
    }
}

fn read_control_plane_store_from_path(path: &Path) -> Result<MockControlPlaneStore> {
    if !path.exists() {
        return Ok(MockControlPlaneStore::default());
    }

    let content = std::fs::read_to_string(path)?;
    if content.trim().is_empty() {
        return Ok(MockControlPlaneStore::default());
    }
    serde_json::from_str::<MockControlPlaneStore>(&content).map_err(AppError::from)
}

fn write_control_plane_store_to_path(path: &Path, store: &MockControlPlaneStore) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(store)?;
    let mut file = open_secret_file(path)?;
    file.write_all(content.as_bytes())?;
    file.flush()?;
    Ok(())
}

fn extract_bearer_token(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    trimmed
        .strip_prefix("Bearer ")
        .or_else(|| trimmed.strip_prefix("bearer "))
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn mask_localhost_api_token(token: &str) -> String {
    if token.len() <= 12 {
        return "已生成".to_string();
    }

    format!("{}…{}", &token[..8], &token[token.len() - 4..])
}

pub fn ensure_localhost_api_token(state: &Arc<Mutex<AppState>>) -> Result<String> {
    let token_path = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        localhost_api_token_path(&state.data_dir)
    };

    if let Some(token) = read_localhost_api_token_from_path(&token_path)? {
        return Ok(token);
    }

    let token = generate_localhost_api_token();
    write_localhost_api_token(&token_path, &token)?;
    Ok(token)
}

pub fn rotate_localhost_api_token(state: &Arc<Mutex<AppState>>) -> Result<String> {
    let token_path = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        localhost_api_token_path(&state.data_dir)
    };

    let token = generate_localhost_api_token();
    write_localhost_api_token(&token_path, &token)?;
    Ok(token)
}

pub fn reveal_localhost_api_token(state: &Arc<Mutex<AppState>>) -> Result<String> {
    ensure_localhost_api_token(state)
}

pub fn get_localhost_api_status(state: &Arc<Mutex<AppState>>) -> Result<LocalhostApiStatusPayload> {
    let node_status = crate::node_gateway::get_node_gateway_status(state)?;
    let (config, runtime_running, runtime_port, last_error, is_recording, is_paused, data_dir) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (
            state.config.clone(),
            state.localhost_api_runtime.running,
            state.localhost_api_runtime.bound_port,
            state.localhost_api_runtime.last_error.clone(),
            state.is_recording,
            state.is_paused,
            state.data_dir.clone(),
        )
    };

    let token = read_localhost_api_token_from_path(&localhost_api_token_path(&data_dir))?;
    let port = runtime_port.unwrap_or(config.localhost_api_port);

    Ok(LocalhostApiStatusPayload {
        enabled: config.localhost_api_enabled,
        running: runtime_running,
        host: LOCALHOST_API_HOST.to_string(),
        port,
        base_url: format!("http://{LOCALHOST_API_HOST}:{port}"),
        token_configured: token.is_some(),
        token_preview: token.as_deref().map(mask_localhost_api_token),
        last_error,
        recording: is_recording,
        paused: is_paused,
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        platform: runtime_platform().to_string(),
        device_id: node_status.device_id,
        device_name: node_status.device_name,
        node_protocol_version: node_status.protocol_version,
        control_plane_enabled: node_status.control_plane_enabled,
        control_plane_endpoint: node_status.control_plane_endpoint,
        control_plane_configured: node_status.control_plane_configured,
        registration_state: node_status.registration_state,
    })
}

fn runtime_platform() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "unknown";
}

fn stop_runtime_locked(runtime: &mut LocalhostApiRuntime) -> Option<oneshot::Sender<()>> {
    runtime.running = false;
    runtime.bound_port = None;
    runtime.shutdown_tx.take()
}

fn record_runtime_error(state: &Arc<Mutex<AppState>>, message: impl Into<String>) {
    if let Ok(mut state) = state.lock() {
        state.localhost_api_runtime.running = false;
        state.localhost_api_runtime.bound_port = None;
        state.localhost_api_runtime.shutdown_tx = None;
        state.localhost_api_runtime.last_error = Some(message.into());
    }
}

pub fn sync_localhost_api_runtime(app: &AppHandle, state: &Arc<Mutex<AppState>>) -> Result<()> {
    let (enabled, port, should_restart, shutdown_tx) = {
        let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let enabled = state.config.localhost_api_enabled;
        let port = if state.config.localhost_api_port == 0 {
            DEFAULT_LOCALHOST_API_PORT
        } else {
            state.config.localhost_api_port
        };

        if !enabled {
            state.localhost_api_runtime.last_error = None;
            let shutdown_tx = stop_runtime_locked(&mut state.localhost_api_runtime);
            return {
                if let Some(shutdown_tx) = shutdown_tx {
                    let _ = shutdown_tx.send(());
                }
                Ok(())
            };
        }

        if state.localhost_api_runtime.running
            && state.localhost_api_runtime.bound_port == Some(port)
        {
            return Ok(());
        }

        let shutdown_tx = stop_runtime_locked(&mut state.localhost_api_runtime);
        (enabled, port, true, shutdown_tx)
    };

    if let Some(shutdown_tx) = shutdown_tx {
        let _ = shutdown_tx.send(());
    }

    if !enabled || !should_restart {
        return Ok(());
    }

    let token = ensure_localhost_api_token(state)?;

    let std_listener = StdTcpListener::bind((LOCALHOST_API_HOST, port)).map_err(|e| {
        let message = format!("启动本地 API 失败: {e}");
        record_runtime_error(state, &message);
        AppError::Config(message)
    })?;
    std_listener.set_nonblocking(true)?;
    let listener = TcpListener::from_std(std_listener)
        .map_err(|e| AppError::Unknown(format!("接管本地 API 监听器失败: {e}")))?;

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    {
        let mut state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        let store_path = mock_control_plane_store_path(&state.data_dir);
        state.localhost_api_runtime.control_plane_store =
            match read_control_plane_store_from_path(&store_path) {
                Ok(store) => store,
                Err(error) => {
                    log::warn!("加载控制面持久化状态失败，已回退为空状态: {error}");
                    MockControlPlaneStore::default()
                }
            };
        state.localhost_api_runtime.running = true;
        state.localhost_api_runtime.bound_port = Some(port);
        state.localhost_api_runtime.last_error = None;
        state.localhost_api_runtime.shutdown_tx = Some(shutdown_tx);
    }

    let app_handle = app.clone();
    let state_handle = state.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) =
            run_localhost_api(listener, shutdown_rx, app_handle, state_handle.clone()).await
        {
            record_runtime_error(&state_handle, format!("本地 API 异常退出: {e}"));
        } else if let Ok(mut state) = state_handle.lock() {
            state.localhost_api_runtime.running = false;
            state.localhost_api_runtime.bound_port = None;
            state.localhost_api_runtime.shutdown_tx = None;
        }
    });

    log::info!(
        "本地 API 已监听在 http://{LOCALHOST_API_HOST}:{port}，token={}",
        mask_localhost_api_token(&token)
    );
    Ok(())
}

async fn run_localhost_api(
    listener: TcpListener,
    mut shutdown_rx: oneshot::Receiver<()>,
    app: AppHandle,
    state: Arc<Mutex<AppState>>,
) -> Result<()> {
    loop {
        tokio::select! {
            _ = &mut shutdown_rx => {
                return Ok(());
            }
            accept_result = listener.accept() => {
                let (stream, _) = accept_result.map_err(|e| AppError::Unknown(format!("接受本地 API 连接失败: {e}")))?;
                let app = app.clone();
                let state = state.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = handle_connection(stream, app, state).await {
                        log::warn!("处理本地 API 请求失败: {e}");
                    }
                });
            }
        }
    }
}

async fn handle_connection(
    mut stream: TcpStream,
    app: AppHandle,
    state: Arc<Mutex<AppState>>,
) -> Result<()> {
    let response = match read_request(&mut stream).await {
        Ok(Some(request)) => route_request(request, &app, &state).await,
        Ok(None) => return Ok(()),
        Err(err) => HttpResponse::error(400, err.to_string()),
    };

    stream.write_all(&response.to_bytes()).await?;
    stream.shutdown().await?;
    Ok(())
}

async fn read_request(stream: &mut TcpStream) -> Result<Option<ParsedRequest>> {
    let mut bytes = Vec::new();
    let mut buffer = [0u8; 4096];
    let header_end;

    loop {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            if bytes.is_empty() {
                return Ok(None);
            }
            return Err(AppError::Config("本地 API 请求头不完整".to_string()));
        }

        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(AppError::Config("本地 API 请求体过大".to_string()));
        }

        if let Some(position) = bytes.windows(4).position(|window| window == b"\r\n\r\n") {
            header_end = position + 4;
            break;
        }
    }

    let header_text = String::from_utf8(bytes[..header_end].to_vec())
        .map_err(|_| AppError::Config("本地 API 请求头不是合法 UTF-8".to_string()))?;
    let mut lines = header_text.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| AppError::Config("本地 API 缺少请求行".to_string()))?;
    let mut request_line_parts = request_line.split_whitespace();
    let method = request_line_parts
        .next()
        .ok_or_else(|| AppError::Config("本地 API 请求方法缺失".to_string()))?
        .to_string();
    let target = request_line_parts
        .next()
        .ok_or_else(|| AppError::Config("本地 API 请求路径缺失".to_string()))?;

    let mut headers = HashMap::new();
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let Some((name, value)) = line.split_once(':') else {
            return Err(AppError::Config("本地 API 请求头格式非法".to_string()));
        };
        headers.insert(name.trim().to_lowercase(), value.trim().to_string());
    }

    let content_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| AppError::Config("本地 API Content-Length 非法".to_string()))?
        .unwrap_or(0);

    if content_length > MAX_BODY_BYTES {
        return Err(AppError::Config("本地 API 请求体超过限制".to_string()));
    }

    while bytes.len() < header_end + content_length {
        let read = stream.read(&mut buffer).await?;
        if read == 0 {
            return Err(AppError::Config("本地 API 请求体不完整".to_string()));
        }
        bytes.extend_from_slice(&buffer[..read]);
        if bytes.len() > MAX_REQUEST_BYTES {
            return Err(AppError::Config("本地 API 请求体过大".to_string()));
        }
    }

    let body = bytes[header_end..header_end + content_length].to_vec();
    let parsed_url = reqwest::Url::parse(&format!("http://localhost{target}"))
        .map_err(|e| AppError::Config(format!("本地 API 请求路径非法: {e}")))?;
    let query = parsed_url
        .query_pairs()
        .into_owned()
        .collect::<HashMap<_, _>>();

    Ok(Some(ParsedRequest {
        method,
        path: parsed_url.path().to_string(),
        query,
        headers,
        body,
    }))
}

async fn route_request(
    request: ParsedRequest,
    app: &AppHandle,
    state: &Arc<Mutex<AppState>>,
) -> HttpResponse {
    match authorize_request(&request, state) {
        Ok(()) => {}
        Err(err) => {
            let status = if matches!(err, AppError::Config(_)) {
                401
            } else {
                500
            };
            return HttpResponse::error(status, err.to_string());
        }
    }

    let result = match (request.method.as_str(), request.path.as_str()) {
        ("GET", "/v1/node/status") => {
            get_localhost_api_status(state).map(|payload| HttpResponse::json(200, &payload))
        }
        ("POST", "/v1/node/register") => parse_json_body::<NodeRegisterRequest>(&request)
            .and_then(|body| {
                if body.protocol_version.trim().is_empty() {
                    return Err(AppError::Config("protocolVersion 不能为空".to_string()));
                }
                if body.device_id.trim().is_empty() {
                    return Err(AppError::Config("deviceId 不能为空".to_string()));
                }

                let device_name = if body.device_name.trim().is_empty() {
                    "Work Review Device".to_string()
                } else {
                    body.device_name.trim().to_string()
                };

                let mut guard = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
                let result = guard
                    .localhost_api_runtime
                    .control_plane_store
                    .register_or_reuse_device(body.device_id.trim(), &device_name);
                let store_path = mock_control_plane_store_path(&guard.data_dir);
                write_control_plane_store_to_path(
                    &store_path,
                    &guard.localhost_api_runtime.control_plane_store,
                )?;
                Ok(HttpResponse::json(
                    200,
                    &serde_json::json!({
                        "installationId": result.installation_id,
                        "deviceToken": result.device_token,
                        "heartbeatIntervalSecs": result.heartbeat_interval_secs,
                    }),
                ))
            }),
        ("POST", "/v1/node/heartbeat") => parse_json_body::<NodeHeartbeatRequest>(&request)
            .and_then(|body| {
                if body.protocol_version.trim().is_empty() {
                    return Err(AppError::Config("protocolVersion 不能为空".to_string()));
                }
                if body.installation_id.trim().is_empty() {
                    return Err(AppError::Config("installationId 不能为空".to_string()));
                }
                if body.device_id.trim().is_empty() {
                    return Err(AppError::Config("deviceId 不能为空".to_string()));
                }

                let token = request
                    .headers
                    .get("authorization")
                    .and_then(|value| extract_bearer_token(value))
                    .ok_or_else(|| AppError::Config("缺少设备 token".to_string()))?;

                let mut guard = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
                match guard
                    .localhost_api_runtime
                    .control_plane_store
                    .validate_and_record_heartbeat(
                        token,
                        body.installation_id.trim(),
                        body.device_id.trim(),
                    ) {
                    Ok(heartbeat_interval_secs) => {
                        let store_path = mock_control_plane_store_path(&guard.data_dir);
                        write_control_plane_store_to_path(
                            &store_path,
                            &guard.localhost_api_runtime.control_plane_store,
                        )?;
                        Ok(HttpResponse::json(
                            200,
                            &serde_json::json!({
                                "heartbeatIntervalSecs": heartbeat_interval_secs,
                            }),
                        ))
                    }
                    Err(ControlPlaneStoreError::Unauthorized(message)) => Ok(HttpResponse::error(401, message)),
                    Err(ControlPlaneStoreError::BadRequest(message)) => Ok(HttpResponse::error(400, message)),
                }
            }),
        ("POST", "/v1/reports/generate") => {
            match parse_json_body::<GenerateReportRequest>(&request) {
                Ok(body) => {
                    commands::generate_report_inner(body.date, body.force, body.locale, app, state)
                        .await
                        .map(|content| {
                            HttpResponse::json(
                                200,
                                &serde_json::json!({
                                    "content": content,
                                }),
                            )
                        })
                }
                Err(err) => Err(err),
            }
        }
        ("POST", "/v1/reports/export-markdown") => parse_json_body::<ExportReportRequest>(&request)
            .and_then(|body| {
                commands::export_report_markdown_inner(
                    body.date,
                    body.content,
                    body.export_dir,
                    state,
                )
                .map(|path| {
                    HttpResponse::json(
                        200,
                        &serde_json::json!({
                            "path": path,
                        }),
                    )
                })
            }),
        _ if request.method == "GET" && request.path.starts_with("/v1/reports/") => {
            let date = request.path.trim_start_matches("/v1/reports/").trim();
            if date.is_empty() {
                Err(AppError::Config("日报日期不能为空".to_string()))
            } else {
                commands::get_saved_report_inner(
                    date.to_string(),
                    request.query.get("locale").cloned(),
                    state,
                )
                .and_then(|report| {
                    report.ok_or_else(|| AppError::Config("未找到该日期的日报".to_string()))
                })
                .map(|report| HttpResponse::json(200, &report))
            }
        }
        ("GET", "/health") => Ok(HttpResponse::text(200, "ok")),
        _ => Ok(HttpResponse::error(404, "未找到本地 API 路径")),
    };

    result.unwrap_or_else(|error| {
        let status = if matches!(error, AppError::Config(_)) {
            400
        } else {
            500
        };
        HttpResponse::error(status, error.to_string())
    })
}

fn parse_json_body<T: for<'de> Deserialize<'de>>(request: &ParsedRequest) -> Result<T> {
    serde_json::from_slice(&request.body)
        .map_err(|e| AppError::Config(format!("本地 API JSON 请求体非法: {e}")))
}

fn request_auth_mode(method: &str, path: &str) -> RequestAuthMode {
    if method == "POST" && path == "/v1/node/register" {
        return RequestAuthMode::None;
    }
    if method == "POST" && path == "/v1/node/heartbeat" {
        return RequestAuthMode::DeviceToken;
    }
    RequestAuthMode::LocalApiToken
}

fn authorize_request(request: &ParsedRequest, state: &Arc<Mutex<AppState>>) -> Result<()> {
    match request_auth_mode(&request.method, &request.path) {
        RequestAuthMode::None => return Ok(()),
        RequestAuthMode::DeviceToken => {
            let has_device_token = request
                .headers
                .get("authorization")
                .and_then(|value| extract_bearer_token(value))
                .is_some();
            if has_device_token {
                return Ok(());
            }
            return Err(AppError::Config("缺少设备 token".to_string()));
        }
        RequestAuthMode::LocalApiToken => {}
    }

    let token_path = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        localhost_api_token_path(&state.data_dir)
    };
    let Some(expected_token) = read_localhost_api_token_from_path(&token_path)? else {
        return Err(AppError::Config("缺少或无效的本地 API token".to_string()));
    };

    let provided = request
        .headers
        .get("authorization")
        .and_then(|value| extract_bearer_token(value));

    if provided == Some(expected_token.as_str()) {
        Ok(())
    } else {
        Err(AppError::Config("缺少或无效的本地 API token".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        extract_bearer_token, mask_localhost_api_token, read_control_plane_store_from_path,
        request_auth_mode, write_control_plane_store_to_path, MockControlPlaneStore,
        RequestAuthMode,
    };

    #[test]
    fn bearer_token解析应忽略前后空白() {
        assert_eq!(extract_bearer_token("Bearer abc123 "), Some("abc123"));
        assert_eq!(extract_bearer_token("bearer xyz"), Some("xyz"));
        assert_eq!(extract_bearer_token("Basic nope"), None);
    }

    #[test]
    fn token预览应避免泄露完整密钥() {
        let masked = mask_localhost_api_token("wr-local-1234567890abcdef");
        assert!(masked.starts_with("wr-local"));
        assert!(masked.contains('…'));
        assert!(!masked.contains("1234567890abcdef"));
    }

    #[test]
    fn 控制面注册应复用同一device_id的安装记录() {
        let mut store = MockControlPlaneStore::default();
        let first = store.register_or_reuse_device("wr-device-a", "机器A");
        let second = store.register_or_reuse_device("wr-device-a", "机器A-新名");

        assert_eq!(first.installation_id, second.installation_id);
        assert_eq!(first.device_token, second.device_token);
        assert_eq!(second.heartbeat_interval_secs, Some(300));
    }

    #[test]
    fn 控制面心跳鉴权应拒绝无效token() {
        let mut store = MockControlPlaneStore::default();
        let registration = store.register_or_reuse_device("wr-device-a", "机器A");

        let result = store.validate_and_record_heartbeat(
            "invalid-token",
            &registration.installation_id,
            "wr-device-a",
        );

        assert!(result.is_err());
    }

    #[test]
    fn 控制面心跳鉴权应接受有效token与匹配安装信息() {
        let mut store = MockControlPlaneStore::default();
        let registration = store.register_or_reuse_device("wr-device-a", "机器A");

        let result = store.validate_and_record_heartbeat(
            &registration.device_token,
            &registration.installation_id,
            "wr-device-a",
        );

        assert_eq!(result.expect("心跳应通过"), Some(300));
    }

    #[test]
    fn 控制面完整链路应支持注册后立即心跳() {
        let mut store = MockControlPlaneStore::default();
        let registration = store.register_or_reuse_device("wr-device-a", "机器A");
        let heartbeat = store.validate_and_record_heartbeat(
            &registration.device_token,
            &registration.installation_id,
            "wr-device-a",
        );

        assert_eq!(heartbeat.expect("注册后心跳应成功"), Some(300));
    }

    #[test]
    fn 控制面状态应支持持久化恢复() {
        let temp_dir = std::env::temp_dir().join(format!(
            "work-review-control-plane-store-test-{}",
            uuid::Uuid::new_v4()
        ));
        let store_path = temp_dir.join("store.json");

        let mut store = MockControlPlaneStore::default();
        let registration = store.register_or_reuse_device("wr-device-a", "机器A");
        write_control_plane_store_to_path(&store_path, &store).expect("写入持久化状态失败");

        let loaded = read_control_plane_store_from_path(&store_path).expect("读取持久化状态失败");
        let recovered = loaded
            .installations
            .get(&registration.installation_id)
            .expect("应恢复已注册安装记录");
        assert_eq!(recovered.device_id, "wr-device-a");
        assert_eq!(recovered.device_token, registration.device_token);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn 鉴权模式应将注册路由标记为免鉴权() {
        assert_eq!(
            request_auth_mode("POST", "/v1/node/register"),
            RequestAuthMode::None
        );
    }

    #[test]
    fn 鉴权模式应将心跳路由标记为设备token鉴权() {
        assert_eq!(
            request_auth_mode("POST", "/v1/node/heartbeat"),
            RequestAuthMode::DeviceToken
        );
    }

    #[test]
    fn 鉴权模式应将其余路由标记为本地api_token鉴权() {
        assert_eq!(
            request_auth_mode("GET", "/v1/node/status"),
            RequestAuthMode::LocalApiToken
        );
    }
}
