<script>
  import { onDestroy } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { formatDurationLocalized, locale, t } from '$lib/i18n/index.js';
  import { appIconStore, getIconCacheKey, preloadAppIcons } from '../stores/iconCache.js';
  import { resolveAppIconSrc } from '../utils/appVisuals.js';

  export let data = [];
  export let mode = 'row';

  // 订阅全局图标缓存
  let appIcons = {};
  const unsubIcons = appIconStore.subscribe(v => appIcons = v);
  onDestroy(() => unsubIcons());

  // 展开/收起状态
  const DEFAULT_COUNT = 8;
  let expanded = false;
  $: currentLocale = $locale;

  // 格式化时长
  function formatDuration(seconds) {
    currentLocale;
    return formatDurationLocalized(seconds, { compact: true });
  }

  // 颜色列表
  const colors = [
    '#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
    '#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1',
  ];

  // 数据变化时预加载图标
  $: if (data) {
    preloadAppIcons(
      displayApps.map(a => ({
        appName: a.app_name,
        executablePath: a.executable_path,
      })),
      invoke
    );
  }

  // 展开时显示全部，收起时显示前 8
  $: displayApps = expanded ? data : data.slice(0, DEFAULT_COUNT);
  $: hasMore = data.length > DEFAULT_COUNT;
  $: maxDuration = displayApps.length > 0 ? Math.max(...displayApps.map(a => a.duration)) : 1;
</script>

<div class="space-y-2" data-locale={currentLocale}>
  {#if mode === 'column'}
    <div class="app-usage-chart__columns rounded-2xl border border-slate-100 bg-white/90 p-4 dark:border-slate-700/60 dark:bg-slate-800/70">
      <div class="app-usage-chart__plot relative rounded-2xl bg-slate-50 px-3 pb-3 pt-4 dark:bg-slate-900/40">
        <div class="pointer-events-none absolute inset-x-3 top-4 bottom-14">
          <div class="absolute inset-x-0 top-0 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
          <div class="absolute inset-x-0 top-1/2 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
          <div class="absolute inset-x-0 bottom-0 border-t border-dashed border-slate-200 dark:border-slate-700/80"></div>
        </div>

        <div class="relative flex h-52 items-end gap-3 overflow-x-auto pb-2">
          {#each displayApps as app, i}
            {@const iconSrc = resolveAppIconSrc(
              app.app_name,
              appIcons[getIconCacheKey({ appName: app.app_name, executablePath: app.executable_path })]
            )}
            <div class="min-w-[5.5rem] flex-1">
              <div class="mb-2 text-center text-[11px] font-medium whitespace-nowrap tabular-nums text-slate-500 dark:text-slate-400">
                {formatDuration(app.duration)}
              </div>
              <div class="mx-auto flex h-32 w-12 items-end rounded-2xl bg-slate-100 p-1 dark:bg-slate-700/50">
                <div
                  class="app-usage-chart__bar w-full rounded-2xl transition-all duration-500"
                  style="height: {Math.max((app.duration / maxDuration) * 100, 6)}%; background-color: {colors[i % colors.length]}; opacity: 0.88"
                ></div>
              </div>
              <div class="mt-3 flex items-center justify-center gap-1.5 px-1">
                {#if iconSrc}
                  <img src={iconSrc} alt="" class="h-5 w-5 rounded-md object-cover" />
                {:else}
                  <span class="inline-flex h-5 w-5 items-center justify-center rounded-md bg-slate-100 text-[10px] text-slate-500 dark:bg-slate-700">{i + 1}</span>
                {/if}
                <span class="min-w-0 truncate text-[11px] font-medium text-slate-600 dark:text-slate-300">{app.app_name}</span>
              </div>
            </div>
          {/each}
        </div>
      </div>
    </div>
  {:else}
    {#each displayApps as app, i}
      {@const iconSrc = resolveAppIconSrc(
        app.app_name,
        appIcons[getIconCacheKey({ appName: app.app_name, executablePath: app.executable_path })]
      )}
      <div class="flex items-center gap-2.5">
        <div class="w-6 h-6 flex-shrink-0 flex items-center justify-center">
          {#if iconSrc}
            <img src={iconSrc} alt="" class="w-5 h-5 rounded-md object-cover" />
          {:else}
            <span class="w-5 h-5 flex items-center justify-center rounded bg-slate-100 dark:bg-slate-700 text-xs text-slate-500">{i + 1}</span>
          {/if}
        </div>
        <span class="w-24 text-xs text-slate-600 dark:text-slate-300 truncate flex-shrink-0">{app.app_name}</span>
        <div class="flex-1 h-4 bg-slate-100 dark:bg-slate-700/50 rounded-full overflow-hidden">
          <div
            class="h-full rounded-full transition-all duration-500"
            style="width: {Math.max((app.duration / maxDuration) * 100, 2)}%; background-color: {colors[i % colors.length]}; opacity: 0.8"
          ></div>
        </div>
        <span class="text-xs text-slate-500 dark:text-slate-400 min-w-[5.5rem] text-right flex-shrink-0 whitespace-nowrap tabular-nums">{formatDuration(app.duration)}</span>
      </div>
    {/each}
  {/if}

  {#if hasMore}
    <button
      class="w-full text-center text-xs text-slate-400 hover:text-primary-500 dark:text-slate-500 dark:hover:text-primary-400 py-1 transition-colors"
      on:click={() => expanded = !expanded}
    >
      {expanded ? t('overview.appUsageCollapse') : t('overview.appUsageExpandAll', { count: data.length })}
    </button>
  {/if}
</div>
