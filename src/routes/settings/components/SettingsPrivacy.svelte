<script>
  import { createEventDispatcher } from 'svelte';
  
  export let config;
  export let runningApps = [];
  export let recentApps = [];
  
  const dispatch = createEventDispatcher();
  
  // 内联输入状态
  let showAppInput = false;
  let selectedApp = '';
  let selectedLevel = 'ignored';
  let showKeywordInput = false;
  let newKeyword = '';
  let showDomainInput = false;
  let newDomain = '';

  // 隐私级别定义 - 使用文字标签避免 emoji 渲染异常
  const privacyLevels = [
    { value: 'full', label: '完全记录', textClass: 'settings-text-success', chipClass: 'settings-chip-success', activeClass: 'settings-segment-success', desc: '记录截图、活动和 OCR 文字' },
    { value: 'anonymized', label: '仅统计时长', textClass: 'settings-text-warn', chipClass: 'settings-chip-warn', activeClass: 'settings-segment-warn', desc: '只统计使用时长，不截图不做 OCR' },
    { value: 'ignored', label: '完全忽略', textClass: 'settings-text-danger', chipClass: 'settings-chip-danger', activeClass: 'settings-segment-danger', desc: '不记录、不统计、不显示' },
  ];

  function addAppRule() {
    if (!selectedApp) return;
    // 检查是否已存在同名规则
    const existingIndex = config.privacy.app_rules.findIndex(r => r.app_name === selectedApp);
    if (existingIndex >= 0) {
      // 更新已有规则
      config.privacy.app_rules[existingIndex].level = selectedLevel;
      config.privacy.app_rules = [...config.privacy.app_rules];
    } else {
      config.privacy.app_rules = [
        ...config.privacy.app_rules,
        { app_name: selectedApp, level: selectedLevel }
      ];
    }
    showAppInput = false;
    selectedApp = '';
    dispatch('change', config);
  }

  function removeAppRule(index) {
    const rules = [...config.privacy.app_rules];
    rules.splice(index, 1);
    config.privacy.app_rules = rules;
    dispatch('change', config);
  }

  function addKeyword() {
    if (!newKeyword.trim()) return;
    // 避免重复添加
    if (config.privacy.excluded_keywords.includes(newKeyword.trim())) {
      newKeyword = '';
      return;
    }
    config.privacy.excluded_keywords = [
      ...config.privacy.excluded_keywords,
      newKeyword.trim()
    ];
    newKeyword = '';
    showKeywordInput = false;
    dispatch('change', config);
  }

  function removeKeyword(index) {
    const keywords = [...config.privacy.excluded_keywords];
    keywords.splice(index, 1);
    config.privacy.excluded_keywords = keywords;
    dispatch('change', config);
  }

  // 域名黑名单管理
  function addDomain() {
    if (!newDomain.trim()) return;
    const domains = config.privacy.excluded_domains || [];
    // 避免重复
    if (domains.includes(newDomain.trim())) {
      newDomain = '';
      return;
    }
    config.privacy.excluded_domains = [...domains, newDomain.trim()];
    newDomain = '';
    showDomainInput = false;
    dispatch('change', config);
  }

  function removeDomain(index) {
    const domains = [...(config.privacy.excluded_domains || [])];
    domains.splice(index, 1);
    config.privacy.excluded_domains = domains;
    dispatch('change', config);
  }

  // 快捷选择应用
  function selectApp(appName) {
    selectedApp = appName;
  }
</script>

<div class="settings-card mb-5">
  <h3 class="settings-card-title">隐私设置</h3>
  <p class="settings-card-desc">所有数据仅存储在本地，不会上传到任何服务器</p>
  
  <div class="settings-section">
    <!-- 应用规则 -->
    <div>
      <div class="flex items-center justify-between mb-1">
        <span class="settings-text">
          应用规则
        </span>
        <button
          on:click={() => showAppInput = !showAppInput}
          class="settings-link-action text-sm"
        >
          {showAppInput ? '收起' : '+ 添加规则'}
        </button>
      </div>
      <p class="settings-muted mb-3">指定应用的记录策略</p>

      {#if showAppInput}
        <div class="settings-panel mb-3 animate-fadeIn">
          <!-- 应用名称输入 -->
          <div class="settings-field mb-3">
            <label for="app-name-input" class="settings-label">输入应用名称或点击下方选择</label>
            <input
              id="app-name-input"
              type="text"
              bind:value={selectedApp}
              class="control-input"
              placeholder="如: Chrome, 1Password, 微信"
            />
          </div>
          <!-- 策略选择：分段按钮 -->
          <div class="settings-field mb-3">
            <span class="settings-label">记录策略</span>
            <div class="flex rounded-lg overflow-hidden border border-slate-200 dark:border-slate-600">
              {#each privacyLevels as level}
                <button
                  on:click={() => selectedLevel = level.value}
                  class="segment-btn
                         {selectedLevel === level.value
                           ? level.activeClass
                           : 'settings-segment-idle'}"
                >
                  {level.label}
                </button>
              {/each}
            </div>
            <p class="text-xs mt-1.5 {privacyLevels.find(l => l.value === selectedLevel)?.textClass || 'settings-subtle'}">
              {privacyLevels.find(l => l.value === selectedLevel)?.desc || ''}
            </p>
          </div>
          
          <!-- 快捷选择 -->
          {#if recentApps.length > 0 || runningApps.length > 0}
          <div class="settings-block">
            {#if recentApps.length > 0}
            <div>
              <span class="settings-subtle block mb-1.5">历史应用</span>
              <div class="flex flex-wrap gap-1.5">
                {#each recentApps.slice(0, 12) as app}
                  <button
                    on:click={() => selectApp(app)}
                    class="settings-chip-button
                           {selectedApp === app 
                             ? 'settings-chip-button-active'
                             : ''}"
                  >
                    {app}
                  </button>
                {/each}
              </div>
            </div>
            {/if}
            
            {#if runningApps.length > 0}
            <div>
              <span class="settings-subtle block mb-1.5">运行中</span>
              <div class="flex flex-wrap gap-1.5">
                {#each runningApps.slice(0, 8) as app}
                  <button
                    on:click={() => selectApp(app)}
                    class="settings-chip-button
                           {selectedApp === app 
                             ? 'settings-chip-button-active'
                             : ''}"
                  >
                    {app}
                  </button>
                {/each}
              </div>
            </div>
            {/if}
          </div>
          {/if}

          <!-- 操作按钮 -->
          <div class="settings-actions mt-4">
            <button
              on:click={() => { showAppInput = false; selectedApp = ''; }}
              class="settings-action-secondary"
            >
              取消
            </button>
            <button
              on:click={addAppRule}
              class="settings-action-primary"
              disabled={!selectedApp}
            >
              添加规则
            </button>
          </div>
        </div>
      {/if}

      <!-- 已有规则列表 -->
      <div class="space-y-2">
        {#each config.privacy.app_rules as rule, i}
          <div class="flex items-center justify-between p-3 bg-slate-50 dark:bg-slate-700/30 rounded-lg group">
            <div class="flex items-center gap-3">
              <span class="text-sm font-medium text-slate-800 dark:text-white">{rule.app_name}</span>
              {#if rule.level === 'full'}
                <span class="settings-chip-success">完全记录</span>
              {:else if rule.level === 'anonymized'}
                <span class="settings-chip-warn">仅统计时长</span>
              {:else}
                <span class="settings-chip-danger">完全忽略</span>
              {/if}
            </div>
            <button
              on:click={() => removeAppRule(i)}
              class="text-xs text-slate-400 hover:text-red-500 opacity-0 group-hover:opacity-100 transition-all"
            >
              删除
            </button>
          </div>
        {/each}
        {#if config.privacy.app_rules.length === 0}
          <p class="settings-empty">暂无特殊规则，所有应用默认完全记录</p>
        {/if}
      </div>
    </div>

    <hr class="border-slate-200 dark:border-slate-700" />

    <!-- 内容过滤（合并敏感词 + 域名黑名单） -->
    <div>
      <span class="settings-text block mb-1">内容过滤</span>
      <p class="settings-muted mb-4">OCR 识别到的文字中包含敏感词时，该段文字不会被保存。黑名单域名的浏览活动不会被记录。</p>
      
      <!-- 敏感词 -->
      <div class="mb-4">
        <div class="flex items-center justify-between mb-2">
          <span class="settings-label">敏感词</span>
          <button
            on:click={() => showKeywordInput = !showKeywordInput}
            class="settings-link-action"
          >
            {showKeywordInput ? '收起' : '+ 添加'}
          </button>
        </div>
        
        {#if showKeywordInput}
          <div class="flex gap-2 mb-2 animate-fadeIn">
            <input
              type="text"
              bind:value={newKeyword}
              class="control-input flex-1"
              placeholder="输入敏感词..."
              on:keydown={(e) => e.key === 'Enter' && addKeyword()}
            />
            <button
              on:click={addKeyword}
              class="settings-action-primary"
            >
              添加
            </button>
          </div>
        {/if}

        <div class="flex flex-wrap gap-1.5">
          {#each config.privacy.excluded_keywords as keyword, i}
            <div class="settings-chip-neutral group">
              <span>{keyword}</span>
              <button
                on:click={() => removeKeyword(i)}
                class="ml-1.5 text-slate-400 hover:text-red-500 opacity-50 group-hover:opacity-100 transition-opacity"
              >
                ×
              </button>
            </div>
          {/each}
          {#if config.privacy.excluded_keywords.length === 0}
            <span class="settings-subtle">暂无敏感词</span>
          {/if}
        </div>
        <!-- 敏感词匹配说明 -->
        <p class="settings-note">OCR 识别内容包含该词时自动过滤，不区分大小写</p>
      </div>

      <!-- 域名黑名单 -->
      <div>
        <div class="flex items-center justify-between mb-2">
          <span class="settings-label">域名黑名单</span>
          <button
            on:click={() => showDomainInput = !showDomainInput}
            class="settings-link-action"
          >
            {showDomainInput ? '收起' : '+ 添加'}
          </button>
        </div>
        
        {#if showDomainInput}
          <div class="flex gap-2 mb-2 animate-fadeIn">
            <input
              type="text"
              bind:value={newDomain}
              class="control-input flex-1"
              placeholder="例如: example.com"
              on:keydown={(e) => e.key === 'Enter' && addDomain()}
            />
            <button
              on:click={addDomain}
              class="settings-action-primary"
            >
              添加
            </button>
          </div>
        {/if}

        <div class="flex flex-wrap gap-1.5">
          {#each (config.privacy.excluded_domains || []) as domain, i}
            <div class="settings-chip-danger group">
              <span>{domain}</span>
              <button
                on:click={() => removeDomain(i)}
                class="ml-1.5 text-red-400 hover:text-red-600 opacity-50 group-hover:opacity-100 transition-opacity"
              >
                ×
              </button>
            </div>
          {/each}
          {#if (config.privacy.excluded_domains || []).length === 0}
            <span class="settings-subtle">暂无黑名单域名</span>
          {/if}
        </div>
        <!-- 域名黑名单格式说明 -->
        <p class="settings-note">填写完整域名如 <code class="settings-code">example.com</code>，该域名下所有页面均不会被记录</p>
      </div>
    </div>
  </div>
</div>
