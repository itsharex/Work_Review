<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { locale, t } from '$lib/i18n/index.js';
  import { showToast } from '$lib/stores/toast.js';

  export let config;

  const dispatch = createEventDispatcher();
  const EMPTY_NODE_GATEWAY_CONFIG = {
    control_plane_enabled: false,
    control_plane_endpoint: null,
    device_name: null,
  };

  let nodeStatus = null;
  let localStatus = null;
  let loading = true;
  let saving = false;
  let actionPending = false;
  let tokenVisible = false;
  let tokenValue = '';
  $: currentLocale = $locale;
  $: settingsCardBeta = t('settings.tabs.beta');

  function normalizeConfig() {
    if (!config.node_gateway || typeof config.node_gateway !== 'object') {
      config.node_gateway = { ...EMPTY_NODE_GATEWAY_CONFIG };
    }
    if (typeof config.node_gateway.control_plane_enabled !== 'boolean') {
      config.node_gateway.control_plane_enabled = false;
    }
    if (
      typeof config.node_gateway.control_plane_endpoint !== 'string' &&
      config.node_gateway.control_plane_endpoint !== null
    ) {
      config.node_gateway.control_plane_endpoint = null;
    }
    if (
      typeof config.node_gateway.device_name !== 'string' &&
      config.node_gateway.device_name !== null
    ) {
      config.node_gateway.device_name = null;
    }
  }

  async function loadStatus() {
    loading = true;
    try {
      const [nextNodeStatus, nextLocalStatus] = await Promise.all([
        invoke('get_node_gateway_status'),
        invoke('get_localhost_api_status'),
      ]);
      nodeStatus = nextNodeStatus;
      localStatus = nextLocalStatus;
    } catch (error) {
      console.error('读取设备节点页面数据失败:', error);
      showToast(t('nodeGatewayPage.loadFailed', { error }), 'error');
    } finally {
      loading = false;
    }
  }

  async function refreshStatus() {
    await loadStatus();
  }

  async function persistConfig(successMessage = null) {
    saving = true;
    try {
      normalizeConfig();
      await invoke('save_config', { config });
      dispatch('change', config);
      await loadStatus();
      if (successMessage) {
        showToast(successMessage, 'success');
      }
      return true;
    } catch (error) {
      console.error('保存设备节点配置失败:', error);
      showToast(t('nodeGatewayPage.saveFailed', { error }), 'error');
      return false;
    } finally {
      saving = false;
    }
  }

  async function toggleControlPlane() {
    config.node_gateway.control_plane_enabled = !config.node_gateway.control_plane_enabled;
    const saved = await persistConfig(t('nodeGatewayPage.saved'));
    if (!saved) {
      config.node_gateway.control_plane_enabled = !config.node_gateway.control_plane_enabled;
    }
  }

  async function registerDevice() {
    actionPending = true;
    try {
      nodeStatus = await invoke('register_node_gateway');
      await loadStatus();
      showToast(t('nodeGatewayPage.saved'), 'success');
    } catch (error) {
      console.error('注册设备失败:', error);
      showToast(t('nodeGatewayPage.saveFailed', { error }), 'error');
      await loadStatus();
    } finally {
      actionPending = false;
    }
  }

  async function sendHeartbeat() {
    actionPending = true;
    try {
      nodeStatus = await invoke('send_node_gateway_heartbeat');
      await loadStatus();
      showToast(t('nodeGatewayPage.saved'), 'success');
    } catch (error) {
      console.error('发送节点心跳失败:', error);
      showToast(t('nodeGatewayPage.saveFailed', { error }), 'error');
      await loadStatus();
    } finally {
      actionPending = false;
    }
  }

  async function revealToken() {
    try {
      tokenValue = await invoke('reveal_localhost_api_token');
      tokenVisible = true;
    } catch (error) {
      console.error('读取本地 API token 失败:', error);
      showToast(t('nodeGatewayPage.tokenRevealFailed', { error }), 'error');
    }
  }

  async function rotateToken() {
    try {
      tokenValue = await invoke('rotate_localhost_api_token');
      tokenVisible = true;
      localStatus = await invoke('get_localhost_api_status');
      showToast(t('nodeGatewayPage.tokenRotated'), 'success');
    } catch (error) {
      console.error('轮换本地 API token 失败:', error);
      showToast(t('nodeGatewayPage.tokenRotateFailed', { error }), 'error');
    }
  }

  async function copyToken() {
    if (!tokenVisible || !tokenValue) {
      await revealToken();
    }
    if (!tokenValue) return;

    try {
      await navigator.clipboard.writeText(tokenValue);
      showToast(t('nodeGatewayPage.tokenCopied'), 'success');
    } catch (error) {
      console.error('复制本地 API token 失败:', error);
      showToast(t('nodeGatewayPage.tokenCopyFailed', { error }), 'error');
    }
  }

  function registrationLabel(state) {
    if (state === 'registered') return t('nodeGatewayPage.statusRegistered');
    if (state === 'error') return t('nodeGatewayPage.statusError');
    if (state === 'configured') return t('nodeGatewayPage.statusConfigured');
    if (state === 'unconfigured') return t('nodeGatewayPage.statusUnconfigured');
    return t('nodeGatewayPage.statusDisabled');
  }

  function formatTimestamp(timestamp) {
    if (!timestamp) {
      return t('nodeGatewayPage.empty');
    }
    try {
      return new Date(timestamp * 1000).toLocaleString(currentLocale);
    } catch {
      return String(timestamp);
    }
  }

  onMount(async () => {
    normalizeConfig();
    await loadStatus();
  });
</script>

<div class="settings-card node-gateway-settings-shell" data-locale={currentLocale}>
  <div class="flex items-center gap-3">
    <h3 class="settings-card-title mb-0">{t('nodeGatewayPage.title')}</h3>
    <span class="inline-flex items-center rounded-full border border-amber-200 bg-amber-50 px-2 py-0.5 text-[10px] font-semibold uppercase tracking-[0.08em] text-amber-700 dark:border-amber-500/30 dark:bg-amber-500/10 dark:text-amber-200">
      {settingsCardBeta}
    </span>
  </div>
  <p class="settings-card-desc">{t('nodeGatewayPage.subtitle')}</p>

  {#if loading}
    <div class="flex justify-center py-10">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>
  {:else if nodeStatus && localStatus}
    <div class="node-gateway-grid grid gap-6 xl:grid-cols-[minmax(0,1.1fr)_minmax(0,0.9fr)]">
      <div class="space-y-6">
        <div class="settings-block">
          <div class="flex items-center justify-between gap-3">
            <div>
              <div class="settings-text">{t('nodeGatewayPage.deviceIdentity')}</div>
              <div class="settings-muted mt-0.5">{t('nodeGatewayPage.protocolVersion')} · {nodeStatus.protocol_version}</div>
            </div>
            <button
              type="button"
              class="segment-btn settings-segment-base"
              on:click={refreshStatus}
              disabled={saving || actionPending}
            >
              {t('nodeGatewayPage.refresh')}
            </button>
          </div>
        </div>

        <div class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="settings-text">{t('nodeGatewayPage.deviceId')}</div>
          <div class="settings-muted mt-1 font-mono break-all">{nodeStatus.device_id}</div>
        </div>

        <label class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="settings-text">{t('nodeGatewayPage.deviceName')}</div>
          <input
            type="text"
            bind:value={config.node_gateway.device_name}
            on:blur={() => persistConfig(t('nodeGatewayPage.saved'))}
            class="mt-2 w-full bg-transparent text-sm text-slate-800 dark:text-white focus:outline-none"
            placeholder={nodeStatus.device_name}
          />
          <p class="settings-note mt-2">{t('nodeGatewayPage.deviceNameHint')}</p>
        </label>

        <div class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="flex items-center justify-between gap-4">
            <div>
              <div class="settings-text">{t('nodeGatewayPage.controlPlaneEnabled')}</div>
              <div class="settings-muted mt-0.5">{registrationLabel(nodeStatus.registration_state)}</div>
            </div>
            <button
              type="button"
              on:click={toggleControlPlane}
              disabled={saving || actionPending}
              class="switch-track {config.node_gateway.control_plane_enabled ? 'bg-primary-500' : 'bg-slate-300 dark:bg-slate-600'} {saving ? 'opacity-60 cursor-not-allowed' : ''}"
            >
              <span class="switch-thumb {config.node_gateway.control_plane_enabled ? 'translate-x-5' : 'translate-x-0'}"></span>
            </button>
          </div>
        </div>

        <label class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="settings-text">{t('nodeGatewayPage.controlPlaneEndpoint')}</div>
          <input
            type="url"
            bind:value={config.node_gateway.control_plane_endpoint}
            on:blur={() => persistConfig(t('nodeGatewayPage.saved'))}
            class="mt-2 w-full bg-transparent text-sm font-mono text-slate-800 dark:text-white focus:outline-none"
            placeholder="https://control-plane.example.com"
          />
          <p class="settings-note mt-2">{t('nodeGatewayPage.controlPlaneEndpointHint')}</p>
        </label>

        <div class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="settings-text">{t('nodeGatewayPage.registrationState')}</div>
          <div class="settings-muted mt-1">{registrationLabel(nodeStatus.registration_state)}</div>
          <div class="settings-note mt-2">{t('nodeGatewayPage.registeredAt')}：{formatTimestamp(nodeStatus.registered_at)}</div>
          <div class="settings-note mt-1">{t('nodeGatewayPage.lastHeartbeatAt')}：{formatTimestamp(nodeStatus.last_heartbeat_at)}</div>
          <div class="mt-3 flex flex-wrap gap-2">
            <button
              type="button"
              class="segment-btn settings-segment-active"
              on:click={registerDevice}
              disabled={saving || actionPending}
            >
              {t('nodeGatewayPage.registerDevice')}
            </button>
            <button
              type="button"
              class="segment-btn settings-segment-base"
              on:click={sendHeartbeat}
              disabled={saving || actionPending}
            >
              {t('nodeGatewayPage.sendHeartbeat')}
            </button>
          </div>
        </div>
      </div>

      <div class="space-y-6">
        <div class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="settings-text">{t('nodeGatewayPage.localApi')}</div>
          <div class="settings-muted mt-1">
            {localStatus.enabled ? t('nodeGatewayPage.localhostEnabled') : t('nodeGatewayPage.localhostDisabled')}
          </div>
          <div class="settings-note mt-3">{t('nodeGatewayPage.localhostAddress')}：{localStatus.base_url}</div>
        </div>

        <div class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="settings-text">{t('nodeGatewayPage.localhostToken')}</div>
          <div class="settings-muted mt-1 font-mono break-all">
            {tokenVisible ? tokenValue || t('nodeGatewayPage.empty') : localStatus.token_preview || t('nodeGatewayPage.empty')}
          </div>
          <p class="settings-note mt-2">{t('nodeGatewayPage.localhostTokenHint')}</p>
          <div class="mt-3 flex flex-wrap gap-2">
            <button type="button" class="segment-btn settings-segment-base" on:click={revealToken}>
              {t('nodeGatewayPage.revealToken')}
            </button>
            <button type="button" class="segment-btn settings-segment-base" on:click={copyToken}>
              {t('nodeGatewayPage.copyToken')}
            </button>
            <button type="button" class="segment-btn settings-segment-active" on:click={rotateToken}>
              {t('nodeGatewayPage.rotateToken')}
            </button>
          </div>
        </div>

        <div class="settings-block border border-slate-200 dark:border-slate-700">
          <div class="settings-text">{t('nodeGatewayPage.lastError')}</div>
          <div class="settings-muted mt-1 break-all">{localStatus.last_error || t('nodeGatewayPage.empty')}</div>
        </div>
      </div>
    </div>
  {/if}
</div>
