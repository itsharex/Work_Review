<script>
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-shell';
  import { marked } from 'marked';
  import { showToast } from '../../lib/stores/toast.js';
  import { cache } from '../../lib/stores/cache.js';
  import { shouldShowPromptAppliedToast } from './reportPromptFeedback.js';

  function getLocalDateString() {
    const now = new Date();
    return `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, '0')}-${String(now.getDate()).padStart(2, '0')}`;
  }

  function getYesterdayDateString() {
    const yesterday = new Date();
    yesterday.setDate(yesterday.getDate() - 1);
    return `${yesterday.getFullYear()}-${String(yesterday.getMonth() + 1).padStart(2, '0')}-${String(yesterday.getDate()).padStart(2, '0')}`;
  }

  let report = null;
  let loading = false;
  let generating = false;
  let error = null;
  let selectedDate = getLocalDateString();
  let isYesterdayReport = false; // 标记是否显示的是昨日日报
  let config = null; // 当前配置
  let lastLoadedDate = '';
  let exportInProgress = false;
  let promptSaving = false;

  // 获取 AI 模式显示名称
  function getAiModeName(mode) {
    const normalizedMode = (mode || '').toString().trim().toLowerCase();
    const modeNames = {
      'local': '基础模板',
      'summary': 'AI 增强',
      'cloud': '云端分析'
    };
    return modeNames[normalizedMode] || mode || '未知';
  }

  function resolveReportMeta(reportData, currentConfig) {
    const fallbackHint = reportData?.content || '';
    let aiMode = reportData?.ai_mode || currentConfig?.ai_mode || '';
    let modelName = reportData?.model_name || null;

    aiMode = (aiMode || '').toString().trim().toLowerCase();

    if (
      fallbackHint.includes('由基础模板生成') ||
      fallbackHint.includes('使用基础模板生成')
    ) {
      aiMode = 'local';
      modelName = null;
    }

    if (!reportData && currentConfig?.ai_mode === 'summary' && currentConfig?.text_model?.model) {
      modelName = currentConfig.text_model.model;
    }

    return { aiMode, modelName };
  }

  async function loadConfig() {
    try {
      config = await invoke('get_config');
    } catch (e) {
      console.error('加载配置失败:', e);
    }
  }

  async function loadReport() {
    // 乐观更新：先显示缓存数据
    let cacheData;
    const unsubscribe = cache.subscribe(c => { cacheData = c; });
    unsubscribe();
    
    if (cacheData.reports[selectedDate]?.data) {
      report = cacheData.reports[selectedDate].data;
      isYesterdayReport = false;
      loading = false;
      
      // 缓存有效则直接返回
      if (cache.isValid(cacheData.reports, selectedDate)) {
        return;
      }
      
      // 后台静默刷新
      try {
        const savedReport = await invoke('get_saved_report', { date: selectedDate });
        if (savedReport) {
          report = savedReport;
          cache.setReport(selectedDate, savedReport);
        }
      } catch (e) {
        console.warn('后台刷新日报失败:', e);
      }
    } else {
      // 首次加载
      loading = true;
      error = null;
      try {
        const savedReport = await invoke('get_saved_report', { date: selectedDate });
        if (savedReport) {
          report = savedReport;
          isYesterdayReport = false;
          cache.setReport(selectedDate, savedReport);
        } else {
          // 如果选择今天且今天无日报，尝试加载昨日日报
          if (selectedDate === getLocalDateString()) {
            const yesterday = getYesterdayDateString();
            const yesterdayReport = await invoke('get_saved_report', { date: yesterday });
            if (yesterdayReport) {
              report = yesterdayReport;
              isYesterdayReport = true;
            } else {
              report = null;
              isYesterdayReport = false;
            }
          } else {
             report = null;
             isYesterdayReport = false;
          }
        }
      } catch (e) {
        error = e.toString();
      } finally {
        loading = false;
      }
    }
  }

  function selectDate(date) {
    if (!date || date === selectedDate) return;
    selectedDate = date;
  }

  async function generateReport(force = true) {
    generating = true;
    error = null;
    try {
      if (config?.ai_mode === 'summary') {
        await persistReportPrompt();
      }
      // 只有强制生成的时候才会覆盖已有日报（后端默认规则，这里force指定传入）。
      await invoke('generate_report', { date: selectedDate, force });
      const savedReport = await invoke('get_saved_report', { date: selectedDate });
      report = savedReport || { date: selectedDate, content: '', created_at: Date.now() / 1000 };
      isYesterdayReport = false;
      cache.setReport(selectedDate, report);

      if (
        shouldShowPromptAppliedToast({
          configAiMode: config?.ai_mode,
          customPrompt: config?.daily_report_custom_prompt,
          reportAiMode: savedReport?.ai_mode,
        })
      ) {
        showToast('已应用附加提示词', 'success');
      }
    } catch (e) {
      error = e.toString();
    } finally {
      generating = false;
    }
  }

  async function persistReportPrompt() {
    if (!config || config.ai_mode !== 'summary' || promptSaving) {
      return;
    }

    promptSaving = true;
    try {
      config.daily_report_custom_prompt = (config.daily_report_custom_prompt || '').trim();
      await invoke('save_config', { config });
    } finally {
      promptSaving = false;
    }
  }

  async function exportReportMarkdown() {
    if (!report) return;

    exportInProgress = true;
    try {
      const exportPath = await invoke('export_report_markdown', {
        date: report.date || selectedDate,
        content: report.content,
      });
      showToast(`日报已导出到 ${exportPath}`, 'success');
    } catch (e) {
      showToast(`导出失败: ${e}`, 'error');
    } finally {
      exportInProgress = false;
    }
  }

  function renderMarkdown(content) {
    return marked(content);
  }

  async function handleReportLinkClick(event) {
    const link = event.target.closest('a[href]');
    if (!link) return;

    const href = link.getAttribute('href');
    if (!href || href.startsWith('#')) return;

    event.preventDefault();
    try {
      await open(href);
    } catch (e) {
      console.error('打开日报链接失败:', e);
    }
  }

  function interceptReportLinks(node) {
    const listener = (event) => {
      handleReportLinkClick(event);
    };

    node.addEventListener('click', listener);

    return {
      destroy() {
        node.removeEventListener('click', listener);
      }
    };
  }

  function formatReportDate(dateStr) {
    const date = new Date(dateStr);
    return date.toLocaleDateString('zh-CN', { year: 'numeric', month: 'long', day: 'numeric', weekday: 'long' });
  }

  $: if (selectedDate && selectedDate !== lastLoadedDate) {
    lastLoadedDate = selectedDate;
    report = null;
    isYesterdayReport = false;
    loadReport();
  }

  $: reportMeta = resolveReportMeta(report, config);

  onMount(() => {
    loadConfig();
  });
</script>

<div class="page-shell">
  <!-- 页面标题 -->
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M8 7h8M8 12h8M8 17h5" />
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M7 3h7l5 5v10a3 3 0 01-3 3H7a3 3 0 01-3-3V6a3 3 0 013-3Z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>
          {selectedDate === getLocalDateString() ? '今日日报' : '历史日报'}
        </h2>
        <p>
        {formatReportDate(selectedDate)}
        {#if config || report}
          <span class="ml-1.5 {reportMeta.aiMode === 'summary' ? 'page-inline-chip-brand' : 'page-inline-chip-muted'}">
            {getAiModeName(reportMeta.aiMode)}
          </span>
          {#if reportMeta.aiMode === 'summary' && reportMeta.modelName}
            <span class="ml-1 page-inline-chip-muted">
              {reportMeta.modelName}
            </span>
          {/if}
        {/if}
        </p>
      </div>
    </div>
    <div class="flex flex-col items-end gap-2">
      <div class="page-toolbar-end">
        <button
          class="page-control-btn {selectedDate === getLocalDateString() ? 'page-control-btn-active' : ''}"
          on:click={() => selectDate(getLocalDateString())}
        >
          今天
        </button>
        <button
          class="page-control-btn {selectedDate === getYesterdayDateString() ? 'page-control-btn-active' : ''}"
          on:click={() => selectDate(getYesterdayDateString())}
        >
          昨天
        </button>
        <input 
          type="date"
          max={getLocalDateString()}
          bind:value={selectedDate}
          class="page-control-input"
        />
      </div>
      <span class="page-help-text">可切换到任意历史日期查看历史日报</span>
      <div class="flex flex-wrap justify-end gap-2">
        {#if report}
          <button
            class="page-action-secondary min-h-10 px-4 py-2"
            on:click={exportReportMarkdown}
            disabled={exportInProgress || !config?.daily_report_export_dir}
            title={config?.daily_report_export_dir ? '' : '请先在设置中配置日报 Markdown 导出目录'}
          >
            {#if exportInProgress}
              <div class="animate-spin rounded-full h-4 w-4 border-2 border-current border-t-transparent"></div>
              导出中...
            {:else}
              导出 Markdown
            {/if}
          </button>
          <button
            class="page-action-warn"
            on:click={() => generateReport(true)}
            disabled={generating}
          >
            {#if generating}
              <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
              生成中...
            {:else}
              <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M4 4v5h.582m15.356 2A8.001 8.001 0 004.582 9m0 0H9m11 11v-5h-.581m0 0a8.003 8.003 0 01-15.357-2m15.357 2H15" />
              </svg>
              重新生成
            {/if}
          </button>
        {/if}
      </div>
    </div>
  </div>

  {#if config}
    <div class="page-card">
      <h3 class="settings-card-title">生成选项</h3>
      {#if config.ai_mode === 'summary'}
        <div class="space-y-3">
          <div>
            <label for="daily-report-custom-prompt" class="settings-label mb-1.5">日报附加提示词</label>
            <textarea
              id="daily-report-custom-prompt"
              bind:value={config.daily_report_custom_prompt}
              on:change={persistReportPrompt}
              rows="4"
              class="control-input resize-y min-h-[110px]"
              placeholder="例如：先给结论再展开；更偏周报口吻；突出产出、风险和下一步。"
            ></textarea>
          </div>
          <p class="settings-note">
            仅 AI 增强模式生效。这里填写的是附加要求，不会覆盖系统默认的日报结构和基础约束。
          </p>
        </div>
      {:else}
        <p class="settings-empty">当前是基础模板模式，附加提示词仅在「AI 增强」模式下生效。</p>
      {/if}
    </div>
  {/if}

  <!-- 日报内容 -->
  {#if loading}
    <div class="empty-state-lg">
      <div class="empty-state-icon">
        <div class="animate-spin rounded-full h-8 w-8 border-2 border-indigo-500 border-t-transparent"></div>
      </div>
      <h3 class="empty-state-title">正在整理日报</h3>
      <p class="empty-state-copy mt-1">正在读取当前日期的历史日报与生成状态...</p>
    </div>
  {:else if error}
    <div class="page-banner-error">
      <div>
        <div class="flex items-center gap-3 text-red-500 mb-2">
        <svg class="w-6 h-6" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <span class="font-medium">生成失败</span>
      </div>
      <p class="text-sm">{error}</p>
      </div>
      <button class="page-action-brand" on:click={() => generateReport(true)}>重试</button>
    </div>
  {:else if report}
    <!-- 昨日日报提示 -->
    {#if isYesterdayReport}
      <div class="page-banner-warning mb-4">
        <div class="flex items-center gap-2 text-sm">
          <svg class="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
          </svg>
          今日日报尚未生成，显示的是昨日日报 ({formatReportDate(report.date)})
        </div>
        <button
          class="page-action-warn min-h-9 px-3 text-xs rounded-lg shadow-none"
          on:click={() => generateReport(false)}
          disabled={generating}
        >
          {#if generating}
            <div class="animate-spin rounded-full h-3 w-3 border-2 border-white border-t-transparent"></div>
          {:else}
            ✨ 生成今日日报
          {/if}
        </button>
      </div>
    {/if}
    <div class="page-card">
      <div class="text-xs text-slate-400 mb-4 flex items-center gap-2">
        <div class="w-1.5 h-1.5 rounded-full {isYesterdayReport ? 'bg-amber-500' : 'bg-emerald-500'}"></div>
        {isYesterdayReport ? '昨日日报 - ' : ''}生成于 {new Date(report.created_at * 1000).toLocaleString('zh-CN')}
      </div>
      <div
        use:interceptReportLinks
        class="markdown-body prose prose-slate dark:prose-invert max-w-none"
      >
        {@html renderMarkdown(report.content)}
      </div>
    </div>
    {:else}
    <div class="empty-state-lg">
      <div class="empty-state-icon !w-16 !h-16 !mb-5 bg-amber-50 dark:bg-amber-950/30">
        <span class="text-3xl">📝</span>
      </div>
      <h3 class="empty-state-title">
        {selectedDate === getLocalDateString() ? '今日暂无日报' : `${selectedDate} 暂无日报`}
      </h3>
      <p class="empty-state-copy mb-5">
        AI 将根据当天的活动记录生成工作总结
      </p>
      <button
        class="page-action-warn min-h-11 px-6 py-3"
        on:click={() => generateReport(false)}
        disabled={generating}
      >
        {#if generating}
          <div class="inline-flex items-center gap-2">
            <div class="animate-spin rounded-full h-4 w-4 border-2 border-white border-t-transparent"></div>
            生成中...
          </div>
        {:else}
          ✨ 生成{selectedDate === getLocalDateString() ? '今日' : '该日'}日报
        {/if}
      </button>
    </div>
  {/if}
</div>

<!-- 表格 / 标题 / 列表等 markdown 样式已统一放到 app.css .markdown-body -->
