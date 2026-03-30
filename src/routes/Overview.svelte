<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import StatsCard from '../lib/components/StatsCard.svelte';
  import AppUsageChart from '../lib/components/AppUsageChart.svelte';
  import { cache } from '../lib/stores/cache.js';
  import { confirm } from '../lib/stores/confirm.js';
  import { showToast } from '../lib/stores/toast.js';
  import { appIconStore, getIconCacheKey, preloadAppIcons } from '../lib/stores/iconCache.js';
  import { resolveAppIconSrc } from '../lib/utils/appVisuals.js';
  import { formatBrowserUrlForDisplay } from '../lib/utils/browserUrl.js';

  let stats = null;
  let loading = true;
  let error = null;
  let unlisten = null;
  let currentTime = new Date();
  let clockInterval;
  let refreshInterval;
  let handleActivityAdded;
  let overviewRefreshPromise = null;
  let overviewRequestId = 0;
  
  let expandedDomains = new Set();
  let editingDomainKey = null;
  let editingSemanticCategory = '';
  let savingDomainKey = null;
  
  // 浏览器统计弹窗
  let selectedBrowser = null;
  
  // 订阅全局图标缓存 store
  let appIcons = {};
  const unsubIcons = appIconStore.subscribe(v => appIcons = v);

  // 响应式图标加载：stats 变化时自动触发
  $: if (stats) {
    if (stats.browser_usage?.length) {
      preloadAppIcons(stats.browser_usage.map(b => ({
        appName: b.browser_name,
        executablePath: b.executable_path,
      })), invoke, { priority: true });
    }

    if (stats.app_usage?.length) {
      preloadAppIcons(stats.app_usage.slice(0, 10).map(a => ({
        appName: a.app_name,
        executablePath: a.executable_path,
      })), invoke);
    }
  }

  function formatDuration(seconds) {
    if (!seconds || seconds <= 0) return '0秒';
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    const secs = seconds % 60;
    if (hours > 0) return `${hours}小时${minutes}分钟`;
    if (minutes > 0) return `${minutes}分钟`;
    return `${secs}秒`;
  }

  function getAppIconSrc(appName, executablePath = null) {
    return resolveAppIconSrc(
      appName,
      appIcons[getIconCacheKey({ appName, executablePath })]
    );
  }

  function getDomainSemanticLabel(domain) {
    return domain?.semantic_category?.trim() || '自动识别';
  }

  function startDomainSemanticEdit(domain) {
    editingDomainKey = domain.domain;
    editingSemanticCategory = domain.semantic_category?.trim() || '';
  }

  function cancelDomainSemanticEdit() {
    editingDomainKey = null;
    editingSemanticCategory = '';
    savingDomainKey = null;
  }

  function findBrowserUsage(browserName, executablePath = null) {
    return stats?.browser_usage?.find((browser) =>
      browser.browser_name === browserName && browser.executable_path === executablePath
    ) || stats?.browser_usage?.find((browser) => browser.browser_name === browserName) || null;
  }

  async function refreshOverviewSelection(browserName, executablePath = null) {
    await loadStats(true);
    selectedBrowser = findBrowserUsage(browserName, executablePath);
  }

  async function saveDomainSemanticRule(domain) {
    const nextCategory = editingSemanticCategory.trim();
    if (!domain || !nextCategory || savingDomainKey === domain.domain) return;
    if ((domain.semantic_category?.trim() || '') === nextCategory) {
      cancelDomainSemanticEdit();
      return;
    }

    const confirmed = await confirm({
      title: '修改网站语义分类',
      message: `将 ${domain.domain} 的网站语义分类改为“${nextCategory}”。按域名生效，并回填该站点的历史记录。是否继续？`,
      confirmText: '确认修改',
      cancelText: '取消',
      tone: 'warning',
    });
    if (!confirmed) return;

    savingDomainKey = domain.domain;
    const browserName = selectedBrowser?.browser_name;
    const executablePath = selectedBrowser?.executable_path || null;

    try {
      const updatedCount = await invoke('set_domain_semantic_rule', {
        domain: domain.domain,
        semanticCategory: nextCategory,
        syncHistory: true,
      });

      await refreshOverviewSelection(browserName, executablePath);
      cancelDomainSemanticEdit();
      showToast(
        `已将 ${domain.domain} 设为“${nextCategory}”，并同步 ${updatedCount} 条历史记录`,
        'success'
      );
    } catch (e) {
      console.error('修改网站语义分类失败:', e);
      showToast(`修改 ${domain.domain} 的网站语义分类失败: ${e}`, 'error');
      savingDomainKey = null;
    }
  }

  async function refreshOverviewStats({ silent = false } = {}) {
    if (overviewRefreshPromise) {
      return overviewRefreshPromise;
    }

    const requestId = ++overviewRequestId;
    overviewRefreshPromise = invoke('get_today_stats')
      .then((newStats) => {
        if (requestId !== overviewRequestId) {
          return;
        }
        stats = newStats;
        cache.setOverview(newStats);
        error = null;
      })
      .catch((e) => {
        if (silent) {
          console.warn('后台刷新失败:', e);
          return;
        }
        error = e.toString();
      })
      .finally(() => {
        overviewRefreshPromise = null;
        loading = false;
      });

    return overviewRefreshPromise;
  }

  async function loadStats(forceRefresh = false) {
    // 乐观更新策略：先显示缓存数据，后台刷新后再更新
    let cacheData;
    const unsubscribe = cache.subscribe(c => { cacheData = c; });
    unsubscribe();
    
    // 如果有缓存数据，立即显示（不显示 loading）
    if (cacheData.overview.data) {
      stats = cacheData.overview.data;
      loading = false;
      
      // 如果缓存有效且非强制刷新，直接返回
      if (!forceRefresh && cache.isValid(cacheData.overview)) {
        return;
      }

      await refreshOverviewStats({ silent: true });
    } else {
      // 首次加载，显示 loading
      loading = true;
      error = null;
      await refreshOverviewStats();
    }
  }

  onMount(async () => {
    loadStats();
    let lastCheckDate = currentTime.getDate();
    clockInterval = setInterval(() => { 
      currentTime = new Date();
      // 跨天检测
      const newDate = currentTime.getDate();
      if (newDate !== lastCheckDate) {
        lastCheckDate = newDate;
        loadStats(true);
      }
    }, 1000);
    
    // 监听 Tauri 截屏事件（后备）
    unlisten = await listen('screenshot-taken', () => loadStats(true));
    
    // 监听全局 activity-added 事件（实时同步）
    handleActivityAdded = () => loadStats(true);
    window.addEventListener('activity-added', handleActivityAdded);
    
    // 定时刷新（30秒）
    refreshInterval = setInterval(loadStats, 30000);
  });

  onDestroy(() => {
    if (unlisten) unlisten();
    if (clockInterval) clearInterval(clockInterval);
    if (refreshInterval) clearInterval(refreshInterval);
    if (handleActivityAdded) window.removeEventListener('activity-added', handleActivityAdded);
    unsubIcons();
  });
</script>

<div class="page-shell">
  <!-- 页面标题 -->
  <div class="page-header">
    <div class="page-title-group">
      <div class="page-title-badge">
        <svg fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.8" d="M4 6.5A2.5 2.5 0 016.5 4H10v6H4V6.5Zm10 0A2.5 2.5 0 0116.5 4H20v6h-6V4Zm-10 11A2.5 2.5 0 016.5 15H10v5H6.5A2.5 2.5 0 014 17.5V15Zm10-2.5H20v2.5A2.5 2.5 0 0117.5 20H14v-5Z" />
        </svg>
      </div>
      <div class="page-title-copy">
        <h2>今日概览</h2>
        <p>
        {new Date().toLocaleDateString('zh-CN', { month: 'long', day: 'numeric', weekday: 'short' })}
        <span class="ml-1.5 font-mono text-xs">{currentTime.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}</span>
        </p>
      </div>
    </div>
    <div class="page-status-chip text-emerald-600 dark:text-emerald-400">
      <span class="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse"></span>
      实时
    </div>
  </div>

  <!-- 统计卡片：始终渲染，内部切换骨架/真实数据 -->
  <div class="grid grid-cols-2 lg:grid-cols-4 gap-3 mb-4">
    {#if loading || !stats}
      {#each [1,2,3,4] as _}
        <div class="min-h-[116px] p-5 rounded-2xl bg-white dark:bg-slate-800/80 border border-slate-100 dark:border-slate-700/60 animate-pulse">
          <div class="flex h-full items-center justify-between gap-4">
            <div class="flex-1">
              <div class="h-3 bg-slate-200 dark:bg-slate-700 rounded w-20"></div>
              <div class="h-8 bg-slate-200 dark:bg-slate-700 rounded w-1/2 mt-6"></div>
            </div>
            <div class="w-11 h-11 rounded-2xl bg-slate-100 dark:bg-slate-700 shrink-0"></div>
          </div>
        </div>
      {/each}
    {:else}
      <StatsCard title="当天活动总时长" value={formatDuration(stats.total_duration)} icon="duration" color="indigo" />
      <StatsCard title="当天办公时长" value={formatDuration(stats.work_time_duration || 0)} icon="focus" color="emerald" />
      <StatsCard title="浏览器" value={formatDuration(stats.browser_duration)} icon="browser" color="blue" />
      <StatsCard title="应用数" value={stats.app_usage.length} icon="apps" color="amber" />
    {/if}
  </div>

  {#if error}
    <div class="page-banner-error mb-4">
      <div>
        <p class="font-semibold">加载概览失败</p>
        <p class="text-sm mt-1">{error}</p>
      </div>
      <button class="page-action-brand" on:click={loadStats}>重试</button>
    </div>
  {/if}

  <!-- 网站访问：始终渲染，加载中显示骨架，无数据显示占位文字 -->
  <div class="page-card mb-4">
    <h3 class="page-section-title">网站访问</h3>
    {#if loading || !stats}
      <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3 animate-pulse">
        {#each [1,2] as _}
          <div class="p-3.5 rounded-xl border border-slate-100 dark:border-slate-700">
            <div class="h-4 bg-slate-200 dark:bg-slate-700 rounded w-3/4 mb-2.5"></div>
            <div class="h-6 bg-slate-200 dark:bg-slate-700 rounded w-1/2 mb-1.5"></div>
            <div class="h-3 bg-slate-100 dark:bg-slate-700/50 rounded w-2/3"></div>
          </div>
        {/each}
      </div>
    {:else if stats.browser_usage && stats.browser_usage.length > 0}
      <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
        {#each stats.browser_usage as browser}
          <button
            class="group text-left p-3.5 rounded-xl border border-slate-100 dark:border-slate-700
                   bg-white dark:bg-slate-800/60
                   hover:border-slate-200 dark:hover:border-slate-600 hover:shadow-sm
                   transition-all duration-200"
            on:click={() => {
              selectedBrowser = browser;
              cancelDomainSemanticEdit();
            }}
          >
            <div class="flex items-center gap-2 mb-1.5">
              {#if getAppIconSrc(browser.browser_name, browser.executable_path)}
                <img src={getAppIconSrc(browser.browser_name, browser.executable_path)} alt="" class="w-6 h-6 rounded-md object-cover" />
              {:else}
                <span class="text-xl">🌐</span>
              {/if}
              <span class="font-medium text-slate-700 dark:text-slate-200 truncate">{browser.browser_name}</span>
            </div>
            <div class="text-lg font-bold text-slate-800 dark:text-white mb-1">
              {formatDuration(browser.duration)}
            </div>
            <div class="flex items-center gap-2 text-xs text-slate-400">
              <span>{browser.domains.length} 站点</span>
              <span>·</span>
              <span>{browser.domains.reduce((sum, d) => sum + d.urls.length, 0)} 页面</span>
            </div>
          </button>
        {/each}
      </div>
    {:else}
      <div class="empty-state-compact">
        <div class="empty-state-icon !w-12 !h-12 !mb-3 shadow-none">
          <span class="text-xl">🌐</span>
        </div>
        <p class="empty-state-copy">今日暂无浏览器访问记录</p>
      </div>
    {/if}
  </div>

  <!-- 应用使用：始终渲染 -->
  <div class="page-card mb-4">
    <h3 class="page-section-title">应用使用</h3>
    {#if loading || !stats}
      <div class="animate-pulse">
        {#each [1,2,3,4] as _}
          <div class="flex items-center gap-3 mb-3">
            <div class="w-7 h-7 rounded bg-slate-200 dark:bg-slate-700 flex-shrink-0"></div>
            <div class="flex-1 h-3 bg-slate-200 dark:bg-slate-700 rounded"></div>
            <div class="w-16 h-3 bg-slate-100 dark:bg-slate-700/50 rounded"></div>
          </div>
        {/each}
      </div>
    {:else if stats.app_usage.length > 0}
      <AppUsageChart data={stats.app_usage} />
    {:else}
      <div class="empty-state-compact">
        <div class="empty-state-icon !w-12 !h-12 !mb-3 shadow-none">
          <span class="text-xl">📊</span>
        </div>
        <p class="empty-state-copy">暂无应用统计数据</p>
      </div>
    {/if}
  </div>
</div>

<!-- 浏览器详情弹窗 -->
{#if selectedBrowser}
<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<div
  class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4"
  role="button"
  tabindex="0"
  on:click|self={() => selectedBrowser = null}
  on:keydown={(e) => e.key === 'Escape' && (selectedBrowser = null)}
>
  <div class="card p-0 max-w-2xl w-full max-h-[85vh] overflow-hidden flex flex-col" role="dialog" aria-modal="true">
    <!-- 弹窗头部 -->
      <div class="flex items-center justify-between p-5 border-b border-slate-200 dark:border-slate-700 bg-gradient-to-r from-slate-50 to-white dark:from-slate-800 dark:to-slate-900">
        <div class="flex items-center gap-3">
        {#if getAppIconSrc(selectedBrowser.browser_name, selectedBrowser.executable_path)}
          <img src={getAppIconSrc(selectedBrowser.browser_name, selectedBrowser.executable_path)} alt="" class="w-8 h-8 rounded-lg object-cover" />
        {:else}
          <span class="text-3xl">🌐</span>
        {/if}
        <div>
          <h3 class="text-lg font-bold text-slate-800 dark:text-white">{selectedBrowser.browser_name}</h3>
          <p class="text-sm text-slate-500 dark:text-slate-400">
            {formatDuration(selectedBrowser.duration)} · {selectedBrowser.domains.length} 站点 · {selectedBrowser.domains.reduce((sum, d) => sum + d.urls.length, 0)} 页面
          </p>
        </div>
      </div>
      <button class="p-2 hover:bg-slate-100 dark:hover:bg-slate-700 rounded-lg transition-colors" on:click={() => selectedBrowser = null}>
        <svg class="w-5 h-5 text-slate-500" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
    
    <!-- 弹窗内容 -->
    <div class="flex-1 overflow-y-auto p-5 space-y-4">
      {#each selectedBrowser.domains as domain}
        <div class="rounded-lg border border-slate-200 dark:border-slate-700 overflow-hidden">
          <!-- 域名头部 -->
          <div class="flex items-center justify-between p-3 bg-slate-50 dark:bg-slate-800/50">
            <div class="flex items-center gap-2">
              <span class="w-2 h-2 rounded-full bg-primary-500"></span>
              <span class="font-medium text-slate-700 dark:text-slate-200">{domain.domain}</span>
              <span class="text-xs text-slate-400 bg-slate-200 dark:bg-slate-700 px-1.5 py-0.5 rounded">
                {domain.urls.length} 页
              </span>
            </div>
            <div class="flex items-center gap-2">
              <span class="text-xs px-2 py-1 rounded-full bg-primary-50 text-primary-700 dark:bg-primary-900/20 dark:text-primary-300">
                当前分类：{getDomainSemanticLabel(domain)}
              </span>
              <button
                class="text-xs px-2 py-1 rounded-full border border-slate-200 dark:border-slate-600 text-slate-600 dark:text-slate-300 hover:border-primary-300 hover:text-primary-600 transition-colors"
                on:click={() => {
                  if (editingDomainKey === domain.domain) {
                    cancelDomainSemanticEdit();
                  } else {
                    startDomainSemanticEdit(domain);
                  }
                }}
              >
                修改分类
              </button>
              <span class="text-sm font-medium text-slate-600 dark:text-slate-300">{formatDuration(domain.duration)}</span>
            </div>
          </div>

          {#if editingDomainKey === domain.domain}
            <div class="px-3 py-3 border-t border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-900/40 space-y-2">
              <label class="block text-xs font-medium text-slate-500 dark:text-slate-400">
                修改分类
              </label>
              <div class="flex flex-col gap-2 md:flex-row">
                <input
                  class="flex-1 rounded-lg border border-slate-200 dark:border-slate-600 bg-white dark:bg-slate-800 px-3 py-2 text-sm text-slate-700 dark:text-slate-100"
                  bind:value={editingSemanticCategory}
                  placeholder="例如：工作跟进、资料阅读、休息娱乐"
                />
                <div class="flex items-center gap-2">
                  <button
                    class="px-3 py-2 rounded-lg bg-primary-500 text-white text-sm disabled:opacity-50"
                    disabled={!editingSemanticCategory.trim() || savingDomainKey === domain.domain}
                    on:click={() => saveDomainSemanticRule(domain)}
                  >
                    {#if savingDomainKey === domain.domain}
                      保存中...
                    {:else}
                      保存
                    {/if}
                  </button>
                  <button
                    class="px-3 py-2 rounded-lg border border-slate-200 dark:border-slate-600 text-sm text-slate-600 dark:text-slate-300"
                    disabled={savingDomainKey === domain.domain}
                    on:click={cancelDomainSemanticEdit}
                  >
                    取消
                  </button>
                </div>
              </div>
            </div>
          {/if}
          
          <!-- URL 列表，支持展开/收起超出的部分 -->
          <div class="divide-y divide-slate-100 dark:divide-slate-700/50">
            {#each (expandedDomains.has(domain.domain) ? domain.urls : domain.urls.slice(0, 10)) as url}
              <div class="flex items-center justify-between p-3 hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
                <div class="flex-1 min-w-0 mr-3">
                  <p
                    class="text-sm text-slate-600 dark:text-slate-300 truncate"
                    title={formatBrowserUrlForDisplay(url.url)}
                  >
                    {formatBrowserUrlForDisplay(url.url)}
                  </p>
                </div>
                <span class="text-xs text-slate-400 whitespace-nowrap">{formatDuration(url.duration)}</span>
              </div>
            {/each}
            {#if domain.urls.length > 10}
              <!-- 展开/收起按钮，让用户可以查看全部 URL -->
              <button
                class="w-full p-3 text-center text-xs text-primary-500 hover:text-primary-600 dark:text-primary-400 hover:bg-primary-50 dark:hover:bg-primary-900/10 transition-colors flex items-center justify-center gap-1"
                on:click={() => {
                  if (expandedDomains.has(domain.domain)) {
                    expandedDomains.delete(domain.domain);
                  } else {
                    expandedDomains.add(domain.domain);
                  }
                  expandedDomains = expandedDomains;
                }}
              >
                {#if expandedDomains.has(domain.domain)}
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M5 15l7-7 7 7"/></svg>
                  收起
                {:else}
                  <svg class="w-3 h-3" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7"/></svg>
                  展开全部 {domain.urls.length} 条
                {/if}
              </button>
            {/if}
          </div>
        </div>
      {/each}
      
      {#if selectedBrowser.domains.length === 0}
        <div class="text-center py-8 text-slate-400">
          <span class="text-3xl">📭</span>
          <p class="mt-2">暂无访问记录</p>
        </div>
      {/if}
    </div>
  </div>
</div>
{/if}
