<script>
  import { invoke } from '@tauri-apps/api/core';
  import { appIconStore, preloadAppIcons } from '../stores/iconCache.js';

  export let data = [];

  // 订阅全局图标缓存
  let appIcons = {};
  const unsubIcons = appIconStore.subscribe(v => appIcons = v);

  import { onDestroy } from 'svelte';
  onDestroy(() => unsubIcons());

  // 格式化时长
  function formatDuration(seconds) {
    const hours = Math.floor(seconds / 3600);
    const minutes = Math.floor((seconds % 3600) / 60);
    if (hours > 0) return `${hours}h${minutes}m`;
    if (minutes > 0) return `${minutes}m`;
    return `${seconds}s`;
  }

  // 颜色列表
  const colors = [
    '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
    '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1',
  ];

  // 数据变化时预加载图标
  $: if (data) {
    preloadAppIcons(data.slice(0, 8).map(a => a.app_name), invoke);
  }

  // 计算最大时长用于计算比例
  $: topApps = data.slice(0, 8);
  $: maxDuration = topApps.length > 0 ? Math.max(...topApps.map(a => a.duration)) : 1;
</script>

<div class="space-y-2.5">
  {#each topApps as app, i}
    <div class="flex items-center gap-2.5">
      <!-- 应用图标或序号 -->
      <div class="w-6 h-6 flex-shrink-0 flex items-center justify-center">
        {#if appIcons[app.app_name]}
          <img src="data:image/png;base64,{appIcons[app.app_name]}" alt="" class="w-5 h-5 rounded" />
        {:else}
          <span class="w-5 h-5 flex items-center justify-center rounded bg-slate-100 dark:bg-slate-700 text-xs text-slate-500">{i + 1}</span>
        {/if}
      </div>
      <!-- 应用名 -->
      <span class="w-24 text-xs text-slate-600 dark:text-slate-300 truncate flex-shrink-0">{app.app_name}</span>
      <!-- 进度条 -->
      <div class="flex-1 h-5 bg-slate-100 dark:bg-slate-700/50 rounded-md overflow-hidden">
        <div
          class="h-full rounded-md transition-all duration-500"
          style="width: {Math.max((app.duration / maxDuration) * 100, 2)}%; background-color: {colors[i % colors.length]}"
        ></div>
      </div>
      <!-- 时长 -->
      <span class="text-xs text-slate-500 dark:text-slate-400 w-14 text-right flex-shrink-0">{formatDuration(app.duration)}</span>
    </div>
  {/each}
</div>
