use crate::error::{AppError, Result};
use crate::AppState;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use uuid::Uuid;

const NODE_IDENTITY_FILE: &str = "node_gateway_identity.json";
const NODE_REGISTRATION_FILE: &str = "node_gateway_registration.json";
const NODE_DEVICE_TOKEN_FILE: &str = "node_gateway_device_token.txt";
pub const NODE_GATEWAY_PROTOCOL_VERSION: &str = "wr-node-gateway/v1alpha1";
const NODE_REQUEST_TIMEOUT_SECS: u64 = 15;
const NODE_CONNECT_TIMEOUT_SECS: u64 = 8;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeIdentity {
    pub device_id: String,
    pub created_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NodeGatewayRegistration {
    pub installation_id: Option<String>,
    pub registered_at: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
    pub heartbeat_interval_secs: Option<u64>,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeGatewayStatusPayload {
    pub protocol_version: String,
    pub device_id: String,
    pub device_name: String,
    pub control_plane_enabled: bool,
    pub control_plane_endpoint: Option<String>,
    pub control_plane_configured: bool,
    pub registration_state: String,
    pub installation_id: Option<String>,
    pub registered_at: Option<i64>,
    pub last_heartbeat_at: Option<i64>,
    pub heartbeat_interval_secs: Option<u64>,
    pub last_error: Option<String>,
}

fn node_identity_path(data_dir: &Path) -> PathBuf {
    data_dir.join(NODE_IDENTITY_FILE)
}

fn node_registration_path(data_dir: &Path) -> PathBuf {
    data_dir.join(NODE_REGISTRATION_FILE)
}

fn node_device_token_path(data_dir: &Path) -> PathBuf {
    data_dir.join(NODE_DEVICE_TOKEN_FILE)
}

pub(crate) fn ensure_node_identity(state: &Arc<Mutex<AppState>>) -> Result<NodeIdentity> {
    let identity_path = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        node_identity_path(&state.data_dir)
    };
    ensure_node_identity_for_path(&identity_path)
}

pub(crate) fn get_node_gateway_status(
    state: &Arc<Mutex<AppState>>,
) -> Result<NodeGatewayStatusPayload> {
    let identity = ensure_node_identity(state)?;
    let (control_plane_enabled, control_plane_endpoint, device_name_override, data_dir) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (
            state.config.node_gateway.control_plane_enabled,
            state.config.node_gateway.control_plane_endpoint.clone(),
            state.config.node_gateway.device_name.clone(),
            state.data_dir.clone(),
        )
    };

    let system_name = system_device_name();
    let device_name = resolve_node_device_name(device_name_override.as_deref(), &system_name);
    let control_plane_configured = control_plane_endpoint.is_some();
    let registration = read_registration_from_path(&node_registration_path(&data_dir))?
        .unwrap_or_default();
    let registration_state =
        derive_registration_state(control_plane_enabled, control_plane_configured, &registration);

    Ok(NodeGatewayStatusPayload {
        protocol_version: NODE_GATEWAY_PROTOCOL_VERSION.to_string(),
        device_id: identity.device_id,
        device_name,
        control_plane_enabled,
        control_plane_endpoint,
        control_plane_configured,
        registration_state: registration_state.to_string(),
        installation_id: registration.installation_id,
        registered_at: registration.registered_at,
        last_heartbeat_at: registration.last_heartbeat_at,
        heartbeat_interval_secs: registration.heartbeat_interval_secs,
        last_error: registration.last_error,
    })
}

pub(crate) async fn register_node_gateway(
    state: &Arc<Mutex<AppState>>,
) -> Result<NodeGatewayStatusPayload> {
    let identity = ensure_node_identity(state)?;
    let (endpoint, device_name, local_api_enabled, local_api_port, data_dir) =
        gather_registration_context(state)?;
    let register_url = node_register_url(&endpoint);
    let client = build_control_plane_client()?;
    let payload = serde_json::json!({
        "protocolVersion": NODE_GATEWAY_PROTOCOL_VERSION,
        "deviceId": identity.device_id,
        "deviceName": device_name,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "platform": platform_label(),
        "localApi": {
            "enabled": local_api_enabled,
            "port": local_api_port,
        }
    });

    let response = client
        .post(register_url)
        .json(&payload)
        .send()
        .await
        .map_err(AppError::Http)?;

    if !response.status().is_success() {
        let message = format!("设备注册失败: HTTP {}", response.status());
        write_registration_error(&data_dir, &message)?;
        return Err(AppError::Config(message));
    }

    let response_body: RegisterNodeResponse = response.json().await.map_err(AppError::Http)?;
    if response_body.installation_id.trim().is_empty() || response_body.device_token.trim().is_empty()
    {
        let message = "设备注册返回内容不完整：缺少 installation_id 或 device_token".to_string();
        write_registration_error(&data_dir, &message)?;
        return Err(AppError::Config(message));
    }

    write_device_token_to_path(
        &node_device_token_path(&data_dir),
        response_body.device_token.trim(),
    )?;

    let registration = NodeGatewayRegistration {
        installation_id: Some(response_body.installation_id.trim().to_string()),
        registered_at: Some(chrono::Utc::now().timestamp()),
        last_heartbeat_at: None,
        heartbeat_interval_secs: response_body.heartbeat_interval_secs,
        last_error: None,
    };
    write_registration_to_path(&node_registration_path(&data_dir), &registration)?;
    get_node_gateway_status(state)
}

pub(crate) async fn send_node_gateway_heartbeat(
    state: &Arc<Mutex<AppState>>,
) -> Result<NodeGatewayStatusPayload> {
    let identity = ensure_node_identity(state)?;
    let (endpoint, device_name, local_api_enabled, local_api_port, data_dir) =
        gather_registration_context(state)?;
    let token_path = node_device_token_path(&data_dir);
    let Some(device_token) = read_device_token_from_path(&token_path)? else {
        let message = "尚未注册设备，无法发送心跳".to_string();
        write_registration_error(&data_dir, &message)?;
        return Err(AppError::Config(message));
    };
    let registration_path = node_registration_path(&data_dir);
    let mut registration = read_registration_from_path(&registration_path)?.unwrap_or_default();
    let installation_id = registration
        .installation_id
        .clone()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| AppError::Config("尚未获得 installation_id，无法发送心跳".to_string()))?;

    let client = build_control_plane_client()?;
    let payload = serde_json::json!({
        "protocolVersion": NODE_GATEWAY_PROTOCOL_VERSION,
        "installationId": installation_id,
        "deviceId": identity.device_id,
        "deviceName": device_name,
        "appVersion": env!("CARGO_PKG_VERSION"),
        "platform": platform_label(),
        "localApi": {
            "enabled": local_api_enabled,
            "port": local_api_port,
        }
    });

    let response = client
        .post(node_heartbeat_url(&endpoint))
        .bearer_auth(device_token)
        .json(&payload)
        .send()
        .await
        .map_err(AppError::Http)?;

    if !response.status().is_success() {
        let message = format!("心跳发送失败: HTTP {}", response.status());
        registration.last_error = Some(message.clone());
        write_registration_to_path(&registration_path, &registration)?;
        return Err(AppError::Config(message));
    }

    let response_body: HeartbeatResponse = response.json().await.map_err(AppError::Http)?;
    registration.last_heartbeat_at = Some(chrono::Utc::now().timestamp());
    if response_body.heartbeat_interval_secs.is_some() {
        registration.heartbeat_interval_secs = response_body.heartbeat_interval_secs;
    }
    registration.last_error = None;
    write_registration_to_path(&registration_path, &registration)?;
    get_node_gateway_status(state)
}

pub(crate) fn resolve_node_device_name(override_name: Option<&str>, fallback_host: &str) -> String {
    let override_name = override_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    if let Some(override_name) = override_name {
        return override_name;
    }

    let fallback_host = fallback_host.trim();
    if !fallback_host.is_empty() {
        return fallback_host.to_string();
    }

    default_device_name()
}

pub(crate) fn ensure_node_identity_for_path(path: &Path) -> Result<NodeIdentity> {
    if let Some(existing) = read_node_identity_from_path(path)? {
        return Ok(existing);
    }

    let identity = NodeIdentity {
        device_id: format!("wr-device-{}", Uuid::new_v4().simple()),
        created_at: chrono::Utc::now().timestamp(),
    };
    write_node_identity_to_path(path, &identity)?;
    Ok(identity)
}

fn read_node_identity_from_path(path: &Path) -> Result<Option<NodeIdentity>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path)?;
    let identity = serde_json::from_str::<NodeIdentity>(&content)?;
    if identity.device_id.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(identity))
    }
}

fn write_node_identity_to_path(path: &Path, identity: &NodeIdentity) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(identity)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn derive_registration_state(
    control_plane_enabled: bool,
    control_plane_configured: bool,
    registration: &NodeGatewayRegistration,
) -> &'static str {
    if !control_plane_enabled {
        "disabled"
    } else if !control_plane_configured {
        "unconfigured"
    } else if registration.last_error.is_some() {
        "error"
    } else if registration
        .installation_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_some()
    {
        "registered"
    } else {
        "configured"
    }
}

fn gather_registration_context(
    state: &Arc<Mutex<AppState>>,
) -> Result<(String, String, bool, u16, PathBuf)> {
    let (node_gateway, localhost_api_enabled, localhost_api_port, data_dir) = {
        let state = state.lock().map_err(|e| AppError::Unknown(e.to_string()))?;
        (
            state.config.node_gateway.clone(),
            state.config.localhost_api_enabled,
            state.config.localhost_api_port,
            state.data_dir.clone(),
        )
    };

    if !node_gateway.control_plane_enabled {
        return Err(AppError::Config("控制面节点模式未启用".to_string()));
    }

    let endpoint = node_gateway
        .control_plane_endpoint
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Config("请先配置控制面地址".to_string()))?
        .trim_end_matches('/')
        .to_string();

    Ok((
        endpoint,
        resolve_node_device_name(node_gateway.device_name.as_deref(), &system_device_name()),
        localhost_api_enabled,
        localhost_api_port,
        data_dir,
    ))
}

fn build_control_plane_client() -> Result<reqwest::Client> {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(NODE_REQUEST_TIMEOUT_SECS))
        .connect_timeout(Duration::from_secs(NODE_CONNECT_TIMEOUT_SECS))
        .build()
        .map_err(AppError::Http)
}

fn node_register_url(endpoint: &str) -> String {
    format!("{}/v1/node/register", endpoint.trim_end_matches('/'))
}

fn node_heartbeat_url(endpoint: &str) -> String {
    format!("{}/v1/node/heartbeat", endpoint.trim_end_matches('/'))
}

fn read_registration_from_path(path: &Path) -> Result<Option<NodeGatewayRegistration>> {
    if !path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(path)?;
    let registration = serde_json::from_str::<NodeGatewayRegistration>(&content)?;
    Ok(Some(registration))
}

fn write_registration_to_path(path: &Path, registration: &NodeGatewayRegistration) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(registration)?;
    std::fs::write(path, content)?;
    Ok(())
}

fn write_registration_error(data_dir: &Path, message: &str) -> Result<()> {
    let path = node_registration_path(data_dir);
    let mut registration = read_registration_from_path(&path)?.unwrap_or_default();
    registration.last_error = Some(message.to_string());
    write_registration_to_path(&path, &registration)
}

fn read_device_token_from_path(path: &Path) -> Result<Option<String>> {
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

fn write_device_token_to_path(path: &Path, token: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = open_secret_file(path)?;
    file.write_all(token.as_bytes())?;
    file.flush()?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct RegisterNodeResponse {
    #[serde(alias = "installationId")]
    installation_id: String,
    #[serde(alias = "deviceToken")]
    device_token: String,
    #[serde(alias = "heartbeatIntervalSecs")]
    #[serde(default)]
    heartbeat_interval_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct HeartbeatResponse {
    #[serde(alias = "heartbeatIntervalSecs")]
    #[serde(default)]
    heartbeat_interval_secs: Option<u64>,
}

fn system_device_name() -> String {
    std::env::var("COMPUTERNAME")
        .ok()
        .or_else(|| std::env::var("HOSTNAME").ok())
        .or_else(read_etc_hostname)
        .unwrap_or_else(default_device_name)
}

#[cfg(target_family = "unix")]
fn read_etc_hostname() -> Option<String> {
    std::fs::read_to_string("/etc/hostname")
        .ok()
        .map(|content| content.trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(not(target_family = "unix"))]
fn read_etc_hostname() -> Option<String> {
    None
}

fn default_device_name() -> String {
    match platform_label() {
        "macOS" => "Work Review Mac".to_string(),
        "Windows" => "Work Review Windows".to_string(),
        "Linux" => "Work Review Linux".to_string(),
        _ => "Work Review Device".to_string(),
    }
}

fn platform_label() -> &'static str {
    #[cfg(target_os = "macos")]
    return "macOS";
    #[cfg(target_os = "windows")]
    return "Windows";
    #[cfg(target_os = "linux")]
    return "Linux";
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    return "Unknown";
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{
        derive_registration_state, ensure_node_identity_for_path, resolve_node_device_name,
        NodeGatewayRegistration,
    };

    fn temp_identity_path(label: &str) -> PathBuf {
        let unique = format!("work-review-node-gateway-{label}-{}", uuid::Uuid::new_v4());
        std::env::temp_dir()
            .join(unique)
            .join("node_identity.json")
    }

    #[test]
    fn 设备身份文件存在时应稳定复用同一device_id() {
        let path = temp_identity_path("stable-device-id");

        let first = ensure_node_identity_for_path(&path).expect("首次生成 device id 失败");
        let second = ensure_node_identity_for_path(&path).expect("二次读取 device id 失败");

        assert_eq!(first.device_id, second.device_id);

        if let Some(parent) = path.parent() {
            let _ = std::fs::remove_dir_all(parent);
        }
    }

    #[test]
    fn 设备显示名应优先使用用户配置覆盖值() {
        let resolved = resolve_node_device_name(Some("  我的工作主机  "), "fallback-host");
        assert_eq!(resolved, "我的工作主机");
    }

    #[test]
    fn 已注册安装实例应把节点状态标记为registered() {
        let registration = NodeGatewayRegistration {
            installation_id: Some("inst_123".to_string()),
            registered_at: Some(1_710_000_000),
            last_heartbeat_at: Some(1_710_000_600),
            heartbeat_interval_secs: Some(300),
            last_error: None,
        };

        assert_eq!(derive_registration_state(true, true, &registration), "registered");
    }

    #[test]
    fn 最近一次注册或心跳失败时应把节点状态标记为error() {
        let registration = NodeGatewayRegistration {
            installation_id: Some("inst_123".to_string()),
            registered_at: Some(1_710_000_000),
            last_heartbeat_at: None,
            heartbeat_interval_secs: None,
            last_error: Some("network timeout".to_string()),
        };

        assert_eq!(derive_registration_state(true, true, &registration), "error");
    }
}
