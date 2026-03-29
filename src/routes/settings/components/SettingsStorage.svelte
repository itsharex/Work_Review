<script>
  import { createEventDispatcher } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { ask, open as openDialog } from '@tauri-apps/plugin-dialog';
  import { cache } from '../../../lib/stores/cache.js';
  import { showToast } from '$lib/stores/toast.js';
  
  export let config;
  export let storageStats = null;
  export let dataDir = '';
  export let defaultDataDir = '';
  
  const dispatch = createEventDispatcher();
  let isClearing = false;
  let isMigrating = false;
  let isCleaningPreviousDir = false;
  let cleanupCandidateDir = '';
  const screenshotModes = [
    {
      value: 'active_window',
      label: '活动窗口所在屏幕',
      description: '默认推荐，只截当前应用所在的那块屏幕',
    },
    {
      value: 'all',
      label: '整桌面拼接截图',
      description: '把所有显示器内容拼成一张完整截图',
    },
  ];

  function clearCache() {
    cache.clear();
    showToast('缓存已清理');
    dispatch('clearCache');
  }

  async function clearOldData() {
    const confirmed = await ask('确认删除今天之前的所有活动记录和截图？此操作不可恢复！', {
      title: '确认清理历史数据',
      kind: 'warning',
    });

    if (!confirmed) {
      return;
    }
    
    isClearing = true;
    try {
      const result = await invoke('clear_old_activities');
      showToast(result?.message || '清理完成');
      cache.clear();
      dispatch('clearCache');
    } catch (e) {
      showToast('清理失败: ' + e, 'error');
    } finally {
      isClearing = false;
    }
  }

  async function migrateToDataDir(targetDir) {
    const nextDir = targetDir?.trim();
    if (!nextDir) {
      return;
    }

    if (nextDir === dataDir) {
      showToast('已是当前数据目录');
      return;
    }

    const confirmed = await ask(
      `将把当前数据迁移到新目录：\n${nextDir}\n\n若目标目录已有 Work Review 历史数据，会被当前数据覆盖。此过程可能持续几秒，是否继续？`,
      {
        title: '确认迁移数据目录',
        kind: 'warning',
      },
    );

    if (!confirmed) {
      return;
    }

    isMigrating = true;
    try {
      const result = await invoke('change_data_dir', { targetDir: nextDir });
      cleanupCandidateDir = result?.oldDataDir || dataDir;
      showToast(result?.message || '数据目录已更新', 'success');
      dispatch('dataDirChanged', result);
    } catch (e) {
      showToast('迁移失败: ' + e, 'error');
    } finally {
      isMigrating = false;
    }
  }

  async function pickDataDir() {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      defaultPath: dataDir || defaultDataDir || undefined,
    });

    if (!selected || Array.isArray(selected)) {
      return;
    }

    await migrateToDataDir(selected);
  }

  async function restoreDefaultDataDir() {
    await migrateToDataDir(defaultDataDir);
  }

  async function openCurrentDataDir() {
    try {
      await invoke('open_data_dir');
    } catch (e) {
      showToast('打开目录失败: ' + e, 'error');
    }
  }

  async function cleanupPreviousDataDir() {
    const targetDir = cleanupCandidateDir?.trim();
    if (!targetDir || isCleaningPreviousDir) {
      return;
    }

    const confirmed = await ask(
      `将清理旧目录中的 Work Review 数据：\n${targetDir}\n\n只会删除应用管理的配置、数据库、截图、OCR 日志等文件；若目录内还有其他文件，会保留它们。是否继续？`,
      {
        title: '确认清理旧目录',
        kind: 'warning',
      },
    );

    if (!confirmed) {
      return;
    }

    isCleaningPreviousDir = true;
    try {
      const result = await invoke('cleanup_old_data_dir', { targetDir });
      cleanupCandidateDir = '';
      showToast(result?.message || '旧目录已清理', 'success');
    } catch (e) {
      showToast('清理旧目录失败: ' + e, 'error');
    } finally {
      isCleaningPreviousDir = false;
    }
  }

  function handleChange() {
    dispatch('change', config);
  }

  async function pickDailyReportExportDir() {
    const selected = await openDialog({
      directory: true,
      multiple: false,
      defaultPath: config.daily_report_export_dir || dataDir || defaultDataDir || undefined,
    });

    if (!selected || Array.isArray(selected)) {
      return;
    }

    config.daily_report_export_dir = selected;
    handleChange();
  }

  function clearDailyReportExportDir() {
    config.daily_report_export_dir = null;
    handleChange();
  }

  // 计算存储使用百分比
  $: usagePercent = storageStats 
    ? Math.min(Math.round((storageStats.total_size_mb / storageStats.storage_limit_mb) * 100), 100) 
    : 0;

  // 使用量颜色
  $: usageColor = usagePercent > 80 ? 'bg-red-500' : usagePercent > 50 ? 'bg-amber-500' : 'bg-emerald-500';
  $: usingDefaultDataDir = dataDir && defaultDataDir && dataDir === defaultDataDir;
  $: if (cleanupCandidateDir && cleanupCandidateDir === dataDir) {
    cleanupCandidateDir = '';
  }
</script>

<!-- 截图与保留 -->
<div class="settings-card mb-5">
  <h3 class="settings-card-title">截图与保留</h3>
  <p class="settings-card-desc">控制截图频率、保留周期和多屏记录方式</p>
  
  <div class="settings-section">
    <!-- 轮询间隔 -->
    <div class="settings-block">
      <div class="flex items-center justify-between">
        <label for="screenshot-interval" class="settings-text">活动轮询间隔</label>
        <span class="settings-value">{config.screenshot_interval}秒</span>
      </div>
      <input
        id="screenshot-interval"
        type="range"
        bind:value={config.screenshot_interval}
        on:change={handleChange}
        min="10"
        max="120"
        step="5"
        class="range-input"
      />
      <div class="flex justify-between text-xs settings-subtle">
        <span>10秒（更精确）</span>
        <span>120秒（更省电）</span>
      </div>
      <p class="settings-note">每隔此时长检测一次当前活动窗口并执行 OCR</p>
    </div>

    <!-- 数据保留 -->
    <div class="settings-block">
      <div class="flex items-center justify-between">
        <label for="retention-days" class="settings-text">数据保留天数</label>
        <span class="settings-value">{config.storage.screenshot_retention_days}天</span>
      </div>
      <input
        id="retention-days"
        type="range"
        bind:value={config.storage.screenshot_retention_days}
        on:change={() => {
          config.storage.metadata_retention_days = config.storage.screenshot_retention_days;
          handleChange();
        }}
        min="1"
        max="90"
        step="1"
        class="range-input"
      />
      <div class="flex justify-between text-xs settings-subtle">
        <span>1天</span>
        <span>90天</span>
      </div>
      <p class="settings-note">超过此天数的活动记录和截图将被自动清理</p>
    </div>

    <div class="settings-block">
      <p class="settings-text mb-2">截图范围</p>
      <div class="flex gap-2">
        {#each screenshotModes as mode}
          <button
            type="button"
            on:click={() => {
              config.storage.screenshot_display_mode = mode.value;
              handleChange();
            }}
            class="flex-1 min-h-16 px-3 py-2.5 rounded-lg text-sm font-medium leading-none transition-all duration-150
                   {config.storage.screenshot_display_mode === mode.value
                     ? 'settings-segment-active'
                     : 'settings-segment-base'}"
          >
            <div class="flex h-full flex-col items-center justify-center gap-1 text-center">
              <div class="leading-none">{mode.label}</div>
              <div class="text-[10px] leading-snug {config.storage.screenshot_display_mode === mode.value ? 'text-white/70' : 'settings-subtle'}">
                {mode.description}
              </div>
            </div>
          </button>
        {/each}
      </div>
      <p class="settings-note">
        默认按活动窗口所在屏幕截图；只有在你确实需要保留整套桌面上下文时，再改成“整桌面拼接截图”。
      </p>
    </div>
  </div>
</div>

<!-- 日报导出 -->
<div class="settings-card mb-5">
  <h3 class="settings-card-title">日报导出</h3>
  <p class="settings-card-desc">设置日报 Markdown 默认下载位置。</p>

  <div class="settings-block">
    <div class="rounded-2xl border border-slate-200/80 bg-slate-50/90 p-4 dark:border-slate-700/80 dark:bg-slate-800/40">
      <p class="settings-text">日报 Markdown 导出目录</p>
      <p class="settings-muted mt-1 break-all">
        {config.daily_report_export_dir || '未设置'}
      </p>
      <p class="settings-note mt-3">设置后，生成日报时会自动导出 YYYY-MM-DD.md。</p>
      <div class="mt-4 flex flex-wrap gap-3">
        <button
          type="button"
          on:click={pickDailyReportExportDir}
          class="settings-action-secondary"
        >
          选择目录
        </button>
        {#if config.daily_report_export_dir}
          <button
            type="button"
            on:click={clearDailyReportExportDir}
            class="settings-action-secondary"
          >
            清空目录
          </button>
        {/if}
      </div>
    </div>
  </div>
</div>

<div class="settings-card mb-5">
  <h3 class="settings-card-title">数据目录与清理</h3>
  <p class="settings-card-desc">管理本地数据位置、容量占用和历史清理</p>

  <div class="settings-section">
    <div class="settings-block">
      <div class="rounded-2xl border border-slate-200/80 bg-slate-50/90 p-4 dark:border-slate-700/80 dark:bg-slate-800/40">
        <div class="grid gap-4 md:grid-cols-2">
          <div>
            <p class="settings-text">当前目录</p>
            <p class="settings-muted mt-1 break-all">{dataDir || '读取中...'}</p>
          </div>
          <div>
            <p class="settings-text">默认目录</p>
            <p class="settings-muted mt-1 break-all">{defaultDataDir || '读取中...'}</p>
          </div>
        </div>

        <div class="mt-4 flex flex-wrap gap-3">
          <button
            on:click={pickDataDir}
            disabled={isMigrating}
            class="settings-action-secondary"
          >
            {#if isMigrating}
              迁移中...
            {:else}
              更改位置
            {/if}
          </button>

          <button
            on:click={openCurrentDataDir}
            disabled={isMigrating}
            class="settings-action-secondary"
          >
            打开当前目录
          </button>

          {#if !usingDefaultDataDir && defaultDataDir}
            <button
              on:click={restoreDefaultDataDir}
              disabled={isMigrating}
              class="settings-action-secondary"
            >
              恢复默认位置
            </button>
          {/if}
        </div>

        <p class="settings-note mt-3">
          建议选择专用空目录。迁移时会复制当前配置、数据库、截图、OCR 日志与背景图。
        </p>

        {#if cleanupCandidateDir}
          <div class="mt-4 rounded-xl border border-amber-200/70 bg-amber-50/90 p-3 dark:border-amber-500/30 dark:bg-amber-950/20">
            <p class="settings-text">旧目录待清理</p>
            <p class="settings-muted mt-1 break-all">{cleanupCandidateDir}</p>
            <p class="settings-note mt-2">
              已切换到新目录。若确认迁移无误，可清理旧目录中的 Work Review 数据；其他非应用文件会保留。
            </p>
            <div class="mt-3 flex flex-wrap gap-3">
              <button
                on:click={cleanupPreviousDataDir}
                disabled={isCleaningPreviousDir || isMigrating}
                class="settings-action-secondary"
              >
                {#if isCleaningPreviousDir}
                  清理中...
                {:else}
                  清理旧目录
                {/if}
              </button>
              <button
                on:click={() => cleanupCandidateDir = ''}
                disabled={isCleaningPreviousDir}
                class="settings-action-secondary"
              >
                稍后处理
              </button>
            </div>
          </div>
        {/if}
      </div>
    </div>

    {#if storageStats}
      <div class="settings-block">
        <div class="rounded-2xl border border-slate-200/80 bg-slate-50/90 p-4 dark:border-slate-700/80 dark:bg-slate-800/40">
          <div class="mb-5">
            <div class="mb-2 flex items-end justify-between">
              <div>
                <span class="text-2xl font-bold text-slate-800 dark:text-white">{storageStats.total_size_mb}</span>
                <span class="settings-muted"> / {storageStats.storage_limit_mb} MB</span>
              </div>
              <span class="text-sm font-medium {usagePercent > 80 ? 'settings-text-danger' : 'settings-muted'}">{usagePercent}%</span>
            </div>
            <div class="h-2.5 w-full overflow-hidden rounded-full bg-slate-100 dark:bg-slate-700">
              <div
                class="h-full rounded-full transition-all duration-500 {usageColor}"
                style="width: {usagePercent}%"
              ></div>
            </div>
          </div>

          <div class="grid grid-cols-3 gap-3">
            <div class="rounded-xl bg-white/70 p-3 text-center ring-1 ring-slate-200/70 dark:bg-slate-900/20 dark:ring-slate-700/70">
              <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.total_files}</p>
              <p class="settings-muted mt-0.5">截图数</p>
            </div>
            <div class="rounded-xl bg-white/70 p-3 text-center ring-1 ring-slate-200/70 dark:bg-slate-900/20 dark:ring-slate-700/70">
              <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.total_size_mb} MB</p>
              <p class="settings-muted mt-0.5">已用空间</p>
            </div>
            <div class="rounded-xl bg-white/70 p-3 text-center ring-1 ring-slate-200/70 dark:bg-slate-900/20 dark:ring-slate-700/70">
              <p class="text-xl font-bold text-slate-800 dark:text-white">{storageStats.retention_days} 天</p>
              <p class="settings-muted mt-0.5">保留期限</p>
            </div>
          </div>
        </div>
      </div>
    {/if}

    <div class="settings-block">
      <div class="flex items-center justify-between rounded-xl bg-slate-50 p-3 dark:bg-slate-700/30">
        <div>
          <p class="settings-text">清理页面缓存</p>
          <p class="settings-muted mt-0.5">解决数据显示异常问题，不影响已保存的数据</p>
        </div>
        <button
          on:click={clearCache}
          class="settings-action-secondary"
        >
          清理缓存
        </button>
      </div>

      <div class="settings-panel-danger flex items-center justify-between">
        <div>
          <p class="settings-text-danger text-sm font-medium">清理历史数据</p>
          <p class="settings-muted mt-0.5">删除今天之前的所有活动记录和截图，不可恢复</p>
        </div>
        <button
          on:click={clearOldData}
          disabled={isClearing}
          class="settings-action-danger"
        >
          {#if isClearing}
            清理中...
          {:else}
            清理历史
          {/if}
        </button>
      </div>
    </div>
  </div>
</div>
