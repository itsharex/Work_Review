<script>
  import { createEventDispatcher, onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { aiStore } from '$lib/stores/ai.js';
  
  export let config;
  export let providers = [];
  
  const dispatch = createEventDispatcher();
  
  // 日报生成模式配置（移除视觉分析模式）
  const aiModes = [
    { 
      value: 'local', 
      icon: '📄',
      label: '基础模板', 
      description: '使用固定格式生成统计报告',
      badge: '可用',
      badgeClass: 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-400',
      requiresText: false
    },
    { 
      value: 'summary', 
      icon: '✨',
      label: 'AI 增强', 
      description: '调用 AI 生成智能工作总结',
      badge: '需配置',
      badgeClass: 'bg-blue-100 text-blue-700 dark:bg-blue-900/40 dark:text-blue-400',
      requiresText: true
    },
  ];

  // 提供商默认配置（从后端获取的 providers 中读取）
  function getProviderDefaults(providerId) {
    const provider = providers.find(p => p.id === providerId);
    return {
      endpoint: provider?.default_endpoint || '',
      model: provider?.default_model || '',
      requiresApiKey: provider?.requires_api_key ?? true
    };
  }

  // 从全局 store 订阅状态
  let textTestStatus = null;
  let textTestMessage = '';
  let textConnectionVerified = false;
  
  // 订阅 aiStore 状态变化
  const unsubscribe = aiStore.subscribe(state => {
    textTestStatus = state.textTestStatus;
    textTestMessage = state.textTestMessage;
    textConnectionVerified = state.textConnectionVerified;
  });

  // 检查文本模型是否已配置（必须测试成功才算已配置）
  $: isTextModelConfigured = textConnectionVerified;
  
  // 检查是否有配置值（用于判断是否需要自动测试）
  $: hasTextModelConfig = !!(config?.text_model?.endpoint && config?.text_model?.model);

  // 响应式计算模式可用性
  $: modeAvailability = aiModes.reduce((acc, mode) => {
    let available = true;
    if (mode.requiresText && !isTextModelConfigured) available = false;
    acc[mode.value] = available;
    return acc;
  }, {});

  // 当前选中的提供商信息
  $: currentProvider = providers.find(p => p.id === config?.text_model?.provider) || providers[0];
  $: requiresApiKey = currentProvider?.requires_api_key ?? true;

  // 每个 provider 的配置缓存（用于切换时保留配置）
  let providerConfigs = {};
  let configInitialized = false;

  // 初始化时缓存当前配置
  $: if (config?.text_model?.provider && !configInitialized) {
    providerConfigs[config.text_model.provider] = {
      endpoint: config.text_model.endpoint,
      model: config.text_model.model,
      api_key: config.text_model.api_key || ''
    };
    configInitialized = true;
  }

  function handleProviderChange(e) {
    const providerId = e.target.value;
    
    // 保存当前 provider 的配置到缓存
    if (config.text_model.provider) {
      providerConfigs[config.text_model.provider] = {
        endpoint: config.text_model.endpoint,
        model: config.text_model.model,
        api_key: config.text_model.api_key || ''
      };
    }
    
    // 尝试从缓存恢复，否则使用默认值
    const defaults = getProviderDefaults(providerId);
    const cached = providerConfigs[providerId];
    
    config.text_model.provider = providerId;
    config.text_model.endpoint = cached?.endpoint || defaults.endpoint;
    config.text_model.model = cached?.model || defaults.model;
    config.text_model.api_key = cached?.api_key || '';
    
    // 重置全局测试状态
    aiStore.reset();
    dispatch('change', config);
  }

  function handleChange() {
    dispatch('change', config);
  }

  async function testTextModel() {
    aiStore.startTesting();
    try {
      const result = await invoke('test_model', { 
        modelConfig: {
          provider: config.text_model.provider,
          endpoint: config.text_model.endpoint,
          api_key: config.text_model.api_key,
          model: config.text_model.model,
        }
      });
      if (result.success) {
        aiStore.setSuccess(result.message + (result.response_time_ms ? ` (${result.response_time_ms}ms)` : ''));
      } else {
        aiStore.setError(result.message);
      }
    } catch (e) {
      aiStore.setError(e.toString());
    }
  }

  // 计算当前配置指纹
  function getConfigHash() {
    if (!config?.text_model) return null;
    const { provider, endpoint, model, api_key } = config.text_model;
    return `${provider}|${endpoint}|${model}|${api_key || ''}`;
  }

  // 组件挂载时：只在配置变化时才自动测试
  onMount(async () => {
    // 延迟等待配置加载
    await new Promise(r => setTimeout(r, 200));
    
    // 获取当前配置指纹
    const currentHash = getConfigHash();
    let lastHash = null;
    const unsub = aiStore.subscribe(s => { lastHash = s.lastTestedConfigHash; });
    unsub();
    
    // 只有配置变化且有配置值时才自动测试
    if (hasTextModelConfig && currentHash !== lastHash) {
      aiStore.setConfigHash(currentHash);
      await testTextModel();
    }
  });
</script>

<!-- 日报生成模式 -->
<section class="mb-8">
  <div class="flex items-center gap-3 mb-4">
    <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-indigo-100 to-purple-100 dark:from-indigo-900/50 dark:to-purple-900/50 flex items-center justify-center">
      <span class="text-indigo-600 dark:text-indigo-400 text-sm">AI</span>
    </div>
    <div>
      <h3 class="text-base font-semibold text-slate-800 dark:text-white">日报生成</h3>
      <p class="text-xs text-slate-500 dark:text-slate-400">时间线和概览始终可用</p>
    </div>
  </div>
  
  <div class="grid grid-cols-2 gap-3">
    {#each aiModes as mode}
      {@const available = modeAvailability[mode.value] ?? false}
      {@const isSelected = config.ai_mode === mode.value}
      <button 
        type="button"
        on:click={() => { if (available) { config.ai_mode = mode.value; handleChange(); } }}
        disabled={!available}
        class="relative p-4 rounded-xl text-left transition-all duration-200
               {!available 
                 ? 'opacity-40 cursor-not-allowed bg-slate-50 dark:bg-slate-800/50' 
                 : isSelected
                   ? 'bg-gradient-to-br from-primary-50 to-primary-100 dark:from-primary-900/30 dark:to-primary-800/20 ring-2 ring-primary-500 shadow-sm' 
                   : 'bg-white dark:bg-slate-800 hover:bg-slate-50 dark:hover:bg-slate-700/50 ring-1 ring-slate-200 dark:ring-slate-700'}"
      >
        <!-- 图标 -->
        <div class="text-2xl mb-2">{mode.icon}</div>
        
        <!-- 标题 -->
        <div class="font-medium text-sm text-slate-800 dark:text-white mb-1">
          {mode.label}
        </div>
        
        <!-- 描述 -->
        <div class="text-xs text-slate-500 dark:text-slate-400 mb-2">
          {mode.description}
        </div>
        
        <!-- 状态徽章 -->
        <div class="text-xs">
          {#if !mode.requiresText}
            <span class="inline-flex items-center gap-1 {mode.badgeClass} px-2 py-0.5 rounded-full font-medium">
              ✓ 可用
            </span>
          {:else if textTestStatus === 'testing'}
            <span class="inline-flex items-center gap-1 bg-slate-100 text-slate-600 dark:bg-slate-700 dark:text-slate-400 px-2 py-0.5 rounded-full font-medium">
              ⏳ 验证中...
            </span>
          {:else if available}
            <span class="inline-flex items-center gap-1 bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-400 px-2 py-0.5 rounded-full font-medium">
              ✓ 已配置
            </span>
          {:else if textTestStatus === 'error' && hasTextModelConfig}
            <span class="inline-flex items-center gap-1 bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400 px-2 py-0.5 rounded-full font-medium">
              ✕ 失败
            </span>
          {:else}
            <span class="inline-flex items-center gap-1 {mode.badgeClass} px-2 py-0.5 rounded-full font-medium">
              {mode.badge}
            </span>
          {/if}
        </div>
        
        <!-- 选中指示器 -->
        {#if isSelected}
          <div class="absolute top-2 right-2 w-2 h-2 rounded-full bg-primary-500"></div>
        {/if}
      </button>
    {/each}
  </div>
</section>

<!-- AI 模型配置 -->
<section class="p-5 rounded-xl bg-white dark:bg-slate-800 ring-1 ring-slate-200 dark:ring-slate-700">
  <div class="flex items-center justify-between mb-5">
    <div class="flex items-center gap-3">
      <div class="w-8 h-8 rounded-lg bg-gradient-to-br from-blue-100 to-cyan-100 dark:from-blue-900/50 dark:to-cyan-900/50 flex items-center justify-center text-xl">
        🤖
      </div>
      <div>
        <h4 class="font-medium text-slate-800 dark:text-white">AI 模型配置</h4>
        <p class="text-xs text-slate-500">选择并配置 AI 服务提供商</p>
      </div>
    </div>
    
    <button
      on:click={testTextModel}
      disabled={textTestStatus === 'testing'}
      class="px-3 py-1.5 text-xs font-medium rounded-lg transition-all
             {textTestStatus === 'success' 
               ? 'bg-emerald-100 text-emerald-700 dark:bg-emerald-900/40 dark:text-emerald-400' 
               : textTestStatus === 'error' 
                 ? 'bg-red-100 text-red-700 dark:bg-red-900/40 dark:text-red-400' 
                 : 'bg-slate-100 hover:bg-slate-200 dark:bg-slate-700 dark:hover:bg-slate-600 text-slate-700 dark:text-slate-300'}"
    >
      {#if textTestStatus === 'testing'}
        <span class="inline-flex items-center gap-1.5">
          <span class="w-3 h-3 border-2 border-current border-t-transparent rounded-full animate-spin"></span>
          测试中...
        </span>
      {:else if textTestStatus === 'success'}
        ✓ 已连接
      {:else if textTestStatus === 'error'}
        ✗ 失败
      {:else}
        测试连接
      {/if}
    </button>
  </div>
  
  {#if textTestMessage}
    <div class="mb-4 px-3 py-2 rounded-lg text-xs {textTestStatus === 'success' ? 'bg-emerald-50 text-emerald-700 dark:bg-emerald-900/20 dark:text-emerald-400' : 'bg-red-50 text-red-700 dark:bg-red-900/20 dark:text-red-400'}">
      {textTestMessage}
    </div>
  {/if}

  <!-- 提供商选择（下拉框） -->
  <div class="grid grid-cols-1 sm:grid-cols-2 gap-3">
    <div>
      <label for="ai-provider" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">提供商</label>
      <select
        id="ai-provider"
        value={config.text_model?.provider || 'ollama'}
        on:change={handleProviderChange}
        class="w-full px-3 py-2 text-sm rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
      >
        {#each providers as provider}
          <option value={provider.id}>{provider.name}</option>
        {/each}
      </select>
    </div>
    
    <div>
      <label for="ai-endpoint" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">API 地址</label>
      <input
        id="ai-endpoint"
        type="text"
        bind:value={config.text_model.endpoint}
        on:change={handleChange}
        class="w-full px-3 py-2 text-sm font-mono rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
        placeholder="http://localhost:11434"
      />
    </div>

    {#if requiresApiKey}
      <div class="sm:col-span-2">
        <label for="ai-apikey" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">API 密钥</label>
        <input
          id="ai-apikey"
          type="password"
          bind:value={config.text_model.api_key}
          on:change={handleChange}
          class="w-full px-3 py-2 text-sm rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
          placeholder="sk-..."
        />
      </div>
    {/if}

    <div class="sm:col-span-2">
      <label for="ai-model" class="block text-xs font-medium text-slate-600 dark:text-slate-400 mb-1.5">模型名称</label>
      <input
        id="ai-model"
        type="text"
        bind:value={config.text_model.model}
        on:change={handleChange}
        class="w-full px-3 py-2 text-sm rounded-lg bg-slate-50 dark:bg-slate-900 border border-slate-200 dark:border-slate-600 focus:ring-2 focus:ring-primary-500 focus:border-transparent"
        placeholder={currentProvider?.default_model || 'qwen2.5'}
      />
      {#if currentProvider?.description}
        <p class="mt-1 text-xs text-slate-400">{currentProvider.description}</p>
      {/if}
    </div>
  </div>
</section>
