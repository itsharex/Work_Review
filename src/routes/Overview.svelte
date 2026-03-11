<script>
  import { onMount, onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import StatsCard from '../lib/components/StatsCard.svelte';
  import AppUsageChart from '../lib/components/AppUsageChart.svelte';
  import { cache } from '../lib/stores/cache.js';
  import { appIconStore, loadAppIcon, preloadAppIcons } from '../lib/stores/iconCache.js';

  let stats = null;
  let loading = true;
  let error = null;
  let unlisten = null;
  let currentTime = new Date();
  let clockInterval;
  let refreshInterval;
  let handleActivityAdded;
  
  let selectedDomain = null;
  
  // 浏览器统计弹窗
  let selectedBrowser = null;
  
  // 订阅全局图标缓存 store
  let appIcons = {};
  const unsubIcons = appIconStore.subscribe(v => appIcons = v);

  // 响应式图标加载：stats 变化时自动触发
  $: if (stats) {
    const names = [];
    if (stats.browser_usage) {
      stats.browser_usage.forEach(b => names.push(b.browser_name));
    }
    if (stats.app_usage) {
      stats.app_usage.slice(0, 10).forEach(a => names.push(a.app_name));
    }
    preloadAppIcons(names, invoke);
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
      
      // 后台静默刷新（不显示 loading 状态）
      try {
        const newStats = await invoke('get_today_stats');
        stats = newStats;
        cache.setOverview(newStats);
      } catch (e) {
        // 静默刷新失败时不显示错误，继续使用缓存
        console.warn('后台刷新失败:', e);
      }
    } else {
      // 首次加载，显示 loading
      loading = true;
      error = null;
      try {
        stats = await invoke('get_today_stats');
        cache.setOverview(stats);
      } catch (e) {
        error = e.toString();
      } finally {
        loading = false;
      }
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
    unlisten = await listen('screenshot-taken', () => setTimeout(() => loadStats(true), 500));
    
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

<div class="p-6 animate-fadeIn">
  <div class="max-w-6xl">
  <!-- 页面标题 -->
  <div class="flex items-center justify-between mb-8">
    <div>
      <h2 class="text-xl font-bold text-slate-800 dark:text-white flex items-center gap-2">
        <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-100 to-purple-100 dark:from-indigo-900/50 dark:to-purple-900/50 flex items-center justify-center">
          <svg class="w-4 h-4 text-indigo-600 dark:text-indigo-400" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
          </svg>
        </div>
        今日概览
      </h2>
      <p class="text-sm text-slate-400 dark:text-slate-500 mt-1 ml-10">
        {new Date().toLocaleDateString('zh-CN', { month: 'long', day: 'numeric', weekday: 'short' })}
        <span class="ml-2 font-mono text-xs">{currentTime.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' })}</span>
      </p>
    </div>
    <div class="flex items-center gap-2 px-3 py-1.5 rounded-full bg-emerald-50 dark:bg-emerald-900/20">
      <span class="w-1.5 h-1.5 bg-emerald-500 rounded-full animate-pulse"></span>
      <span class="text-xs font-medium text-emerald-600 dark:text-emerald-400">实时</span>
    </div>
  </div>

  {#if loading}
    <div class="flex items-center justify-center h-64">
      <div class="animate-spin rounded-full h-8 w-8 border-b-2 border-primary-500"></div>
    </div>
  {:else if error}
    <div class="card p-6 text-center">
      <p class="text-red-500">{error}</p>
      <button class="btn btn-primary mt-4" on:click={loadStats}>重试</button>
    </div>
  {:else if stats}
    <!-- 统计卡片 -->
    <div class="grid grid-cols-2 lg:grid-cols-4 gap-4 mb-8">
      <StatsCard title="当天活动总时长" value={formatDuration(stats.total_duration)} icon="⏱️" color="indigo" />
      <StatsCard title="当天办公时长" value={formatDuration(stats.work_time_duration || 0)} icon="🏢" color="emerald" />
      <StatsCard title="浏览器" value={formatDuration(stats.browser_duration)} icon="🌐" color="blue" />
      <StatsCard title="应用数" value={stats.app_usage.length} icon="🖥️" color="amber" />
    </div>

    <!-- 网站访问 -->
    {#if stats.browser_usage && stats.browser_usage.length > 0}
    <div class="p-5 rounded-2xl bg-white dark:bg-slate-800/80 ring-1 ring-slate-200/50 dark:ring-slate-700/50 mb-6">
      <h3 class="text-sm font-semibold text-slate-700 dark:text-slate-200 mb-4 flex items-center gap-2">
        <div class="w-6 h-6 rounded-lg bg-gradient-to-br from-green-100 to-emerald-100 dark:from-green-900/50 dark:to-emerald-900/50 flex items-center justify-center">
          <span class="text-green-600 dark:text-green-400 text-xs">🌐</span>
        </div>
        网站访问
      </h3>
      <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-4 gap-3">
        {#each stats.browser_usage as browser}
          <button
            class="group text-left p-4 rounded-xl border border-slate-200 dark:border-slate-700 
                   bg-gradient-to-br from-white to-slate-50 dark:from-slate-800 dark:to-slate-900
                   hover:border-primary-300 dark:hover:border-primary-700 hover:shadow-lg
                   transition-all duration-200"
            on:click={() => selectedBrowser = browser}
          >
            <div class="flex items-center gap-2 mb-2">
              {#if appIcons[browser.browser_name]}
                <img src="data:image/png;base64,{appIcons[browser.browser_name]}" alt="" class="w-6 h-6 rounded" />
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
    </div>
    {/if}

    <!-- 应用使用 -->
    <div class="p-5 rounded-2xl bg-white dark:bg-slate-800/80 ring-1 ring-slate-200/50 dark:ring-slate-700/50 mb-6">
      <h3 class="text-lg font-semibold text-slate-800 dark:text-white mb-4">应用使用</h3>
      {#if stats.app_usage.length > 0}
        <AppUsageChart data={stats.app_usage} />
      {:else}
        <p class="text-slate-500 dark:text-slate-400 text-center py-8">暂无数据</p>
      {/if}
    </div>
  {:else}
    <div class="card p-6 text-center">
      <p class="text-slate-500 dark:text-slate-400">暂无数据，开始使用后将显示统计信息</p>
    </div>
  {/if}
  </div>
</div>

<!-- 浏览器详情弹窗 -->
{#if selectedBrowser}
<div
  class="fixed inset-0 bg-black/60 backdrop-blur-sm flex items-center justify-center z-50 p-4"
  role="button"
  tabindex="0"
  on:click={() => selectedBrowser = null}
  on:keydown={(e) => e.key === 'Escape' && (selectedBrowser = null)}
>
  <div class="card p-0 max-w-2xl w-full max-h-[85vh] overflow-hidden flex flex-col" role="dialog" aria-modal="true" on:click|stopPropagation on:keydown|stopPropagation>
    <!-- 弹窗头部 -->
    <div class="flex items-center justify-between p-5 border-b border-slate-200 dark:border-slate-700 bg-gradient-to-r from-slate-50 to-white dark:from-slate-800 dark:to-slate-900">
      <div class="flex items-center gap-3">
        {#if appIcons[selectedBrowser.browser_name]}
          <img src="data:image/png;base64,{appIcons[selectedBrowser.browser_name]}" alt="" class="w-8 h-8 rounded" />
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
            <span class="text-sm font-medium text-slate-600 dark:text-slate-300">{formatDuration(domain.duration)}</span>
          </div>
          
          <!-- URL 列表 -->
          <div class="divide-y divide-slate-100 dark:divide-slate-700/50">
            {#each domain.urls.slice(0, 10) as url}
              <div class="flex items-center justify-between p-3 hover:bg-slate-50 dark:hover:bg-slate-800/30 transition-colors">
                <div class="flex-1 min-w-0 mr-3">
                  <p class="text-sm text-slate-600 dark:text-slate-300 truncate" title={url.url}>
                    {url.url}
                  </p>
                </div>
                <span class="text-xs text-slate-400 whitespace-nowrap">{formatDuration(url.duration)}</span>
              </div>
            {/each}
            {#if domain.urls.length > 10}
              <div class="p-3 text-center text-xs text-slate-400">
                还有 {domain.urls.length - 10} 个页面未显示
              </div>
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

