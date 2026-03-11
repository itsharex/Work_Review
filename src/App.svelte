<script>
  import { onMount } from 'svelte';
  import Router from 'svelte-spa-router';
  import Sidebar from './lib/components/Sidebar.svelte';
  import Overview from './routes/Overview.svelte';
  import Timeline from './routes/timeline/Timeline.svelte';
  import Summary from './routes/timeline/Summary.svelte';
  import Report from './routes/report/Report.svelte';
  import Settings from './routes/settings/Settings.svelte';
  import About from './routes/about/About.svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { listen } from '@tauri-apps/api/event';
  import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
  import { cache, getLocalDate } from './lib/stores/cache.js';

  const appWindow = getCurrentWebviewWindow();

  // 窗口控制函数
  async function closeWindow() {
    await appWindow.hide();
  }

  async function minimizeWindow() {
    await appWindow.minimize();
  }

  async function maximizeWindow() {
    const isMaximized = await appWindow.isMaximized();
    if (isMaximized) {
      await appWindow.unmaximize();
    } else {
      await appWindow.maximize();
    }
  }

  // 预加载核心数据
  async function preloadApp() {
    const today = getLocalDate();
    
    // 并行预加载：概览、时间线(今天)、日报(今天)
    Promise.all([
      // 1. 概览
      invoke('get_today_stats').then(stats => cache.setOverview(stats)),
      // 2. 时间线 (今天) - 仅预加载前 20 条
      Promise.all([
        invoke('get_timeline', { date: today, limit: 20, offset: 0 }),
        invoke('get_hourly_summaries', { date: today })
      ]).then(([activities, summaries]) => cache.setTimeline(today, activities, summaries)),
      // 3. 日报 (今天) - 检查是否已存在
      invoke('get_saved_report', { date: today }).then(report => {
        if (report) cache.setReport(today, report);
      })
    ]).then(() => {
    }).catch(e => {
      console.warn('预加载部分失败:', e);
    });
  }

  const routes = {
    '/': Overview,
    '/timeline': Timeline,
    '/timeline/summary': Summary,
    '/report': Report,
    '/settings': Settings,
    '/about': About,
  };

  let theme = 'system';
  let isDark = false;
  let isRecording = true;
  let isPaused = false;
  let platform = '';

  function detectSystemTheme() {
    return window.matchMedia('(prefers-color-scheme: dark)').matches;
  }

  function applyTheme(newTheme) {
    theme = newTheme;
    isDark = theme === 'system' ? detectSystemTheme() : theme === 'dark';
    
    if (isDark) {
      document.documentElement.classList.add('dark');
    } else {
      document.documentElement.classList.remove('dark');
    }
  }

  async function handleThemeChange(event) {
    const newTheme = event.detail;
    applyTheme(newTheme);
    
    try {
      const config = await invoke('get_config');
      config.theme = newTheme;
      await invoke('save_config', { config });
    } catch (e) {
      console.error('保存主题配置失败:', e);
    }
  }

  onMount(async () => {
    // 获取平台信息
    try {
      platform = await invoke('get_platform');
    } catch (e) {
      console.error('获取平台信息失败:', e);
    }

    // 加载配置并应用主题
    let config;
    try {
      config = await invoke('get_config');
      applyTheme(config.theme || 'system');
    } catch (e) {
      console.error('加载配置失败:', e);
      applyTheme('system');
      config = { work_end_hour: 18 };
    }

    try {
      const [recording, paused] = await invoke('get_recording_state');
      isRecording = recording;
      isPaused = paused;
    } catch (e) {
      console.error('获取录制状态失败:', e);
    }

    window.matchMedia('(prefers-color-scheme: dark)').addEventListener('change', () => {
      if (theme === 'system') applyTheme('system');
    });
    
    // 启动预加载
    preloadApp();

    // 日报自动生成检测：每分钟检查一次
    let lastAutoGenDate = null;  // 防止同一天重复触发
    const autoReportTimer = setInterval(async () => {
      const now = new Date();
      const currentHour = now.getHours();
      const today = getLocalDate();
      
      // 检查是否达到工作结束时间
      const workEndHour = config?.work_end_hour ?? 18;
      
      // 条件：当前小时等于工作结束时间，且今天未自动生成过
      if (currentHour === workEndHour && lastAutoGenDate !== today) {
        try {
          // 检查今日是否已有日报
          const existingReport = await invoke('get_saved_report', { date: today });
          if (!existingReport) {
            await invoke('generate_report', { date: today, force: false });
            cache.invalidate('report');
            lastAutoGenDate = today;
          } else {
            lastAutoGenDate = today;  // 已有日报，标记今天不再触发
          }
        } catch (e) {
          console.warn('日报自动生成失败:', e);
        }
      }
    }, 60000);  // 每分钟检查一次

    const unlisten = await listen('screenshot-taken', (event) => {
      
      // 1. 增量更新时间线缓存
      cache.addActivity(event.payload);
      
      // 2. 使概览缓存过期（下次访问或当前页面监听时刷新）
      cache.invalidate('overview');
      
      // 3. 发射自定义事件，通知当前页面实时更新
      window.dispatchEvent(new CustomEvent('activity-added', { detail: event.payload }));
    });

    return () => {
      unlisten();
      clearInterval(autoReportTimer);
    };
  });
</script>

<div class="flex h-screen bg-slate-50 dark:bg-slate-900 overflow-hidden relative">
  <!-- 
    全局顶部拖拽层 (Invisible Drag Layer)
    1. 覆盖在所有内容之上 (z-50)
    2. 负责处理窗口拖动 (-webkit-app-region: drag)
    3. 按钮区域排除拖动 (-webkit-app-region: no-drag)
  -->
  <div class="absolute top-0 left-0 w-full h-7 z-50" style="-webkit-app-region: drag;">
    <!-- 仅 Windows/Linux 平台显示自定义窗口控制按钮，macOS 使用原生控件 -->
    {#if platform && platform !== 'macos'}
    <!-- Windows/Linux 窗口控制按钮 (Left aligned) -->
    <div class="pl-4 pb-1 flex items-center gap-2 group/titlebar">
      <!-- Close -->
      <button 
        on:click={closeWindow}
        class="w-3 h-3 rounded-full bg-[#FF5F57] border-[0.5px] border-[#E0443E] flex items-center justify-center focus:outline-none active:brightness-90 transition-all"
        style="-webkit-app-region: no-drag;"
        title="关闭"
      >
        <svg class="w-2 h-2 text-[#4D0000] opacity-0 group-hover/titlebar:opacity-100" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
          <path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
      
      <!-- Minimize -->
      <button 
        on:click={minimizeWindow}
        class="w-3 h-3 rounded-full bg-[#FFBD2E] border-[0.5px] border-[#D89614] flex items-center justify-center focus:outline-none active:brightness-90 transition-all"
        style="-webkit-app-region: no-drag;"
        title="最小化"
      >
        <svg class="w-2 h-2 text-[#995700] opacity-0 group-hover/titlebar:opacity-100" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
          <path stroke-linecap="round" stroke-linejoin="round" d="M5 12h14" />
        </svg>
      </button>
      
      <!-- Maximize -->
      <button 
        on:click={maximizeWindow}
        class="w-3 h-3 rounded-full bg-[#28C840] border-[0.5px] border-[#1AAB29] flex items-center justify-center focus:outline-none active:brightness-90 transition-all"
        style="-webkit-app-region: no-drag;"
        title="最大化"
      >
        <svg class="w-2 h-2 text-[#006500] opacity-0 group-hover/titlebar:opacity-100" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="3">
          <path stroke-linecap="round" stroke-linejoin="round" d="M4.5 10.5L12 3m0 0l7.5 7.5M12 3v18" />
        </svg>
      </button>
    </div>
    {/if}
  </div>

  <!-- 左侧边栏 -->
  <div class="w-56 bg-white/80 dark:bg-slate-900/90 backdrop-blur-xl border-r border-slate-200/50 dark:border-slate-700/50 flex flex-col pt-2">
    <Sidebar {isRecording} {isPaused} {theme} on:themeChange={handleThemeChange} />
  </div>
  
  <!-- 右侧主内容区域 -->
  <div class="flex-1 flex flex-col overflow-hidden pt-2">
    <main class="flex-1 overflow-auto">
      <Router {routes} />
    </main>
  </div>
</div>
