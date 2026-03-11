<script>
  export let title = '';
  export let value = '';
  export let icon = '';
  export let color = 'indigo';
  export let href = null;

  // 中性柔和配色方案（蓝灰色系，专业感）
  const gradients = {
    indigo: 'from-slate-100 to-blue-100 dark:from-slate-800 dark:to-blue-900/40',
    emerald: 'from-emerald-50 to-teal-100 dark:from-emerald-900/40 dark:to-teal-900/40',
    amber: 'from-amber-50 to-yellow-100 dark:from-amber-900/40 dark:to-yellow-900/40',
    rose: 'from-slate-100 to-gray-100 dark:from-slate-800 dark:to-gray-800',
    blue: 'from-blue-50 to-sky-100 dark:from-blue-900/40 dark:to-sky-900/40',
    violet: 'from-indigo-50 to-blue-100 dark:from-indigo-900/40 dark:to-blue-900/40',
  };

  // 柔和阴影颜色
  const shadowColors = {
    indigo: 'shadow-slate-200/30 dark:shadow-slate-900/20',
    emerald: 'shadow-emerald-200/30 dark:shadow-emerald-900/20',
    amber: 'shadow-amber-200/30 dark:shadow-amber-900/20',
    rose: 'shadow-slate-200/30 dark:shadow-slate-900/20',
    blue: 'shadow-blue-200/30 dark:shadow-blue-900/20',
    violet: 'shadow-indigo-200/30 dark:shadow-indigo-900/20',
  };
</script>

{#if href}
  <a 
    {href}
    class="block p-5 rounded-2xl bg-white dark:bg-slate-800/80 ring-1 ring-slate-200/50 dark:ring-slate-700/50 
           transition-all duration-300 hover:shadow-xl hover:-translate-y-1 cursor-pointer group"
  >
    <div class="flex items-center gap-4">
      <div class="w-11 h-11 rounded-xl bg-gradient-to-br {gradients[color]} flex items-center justify-center text-white text-lg shadow-lg {shadowColors[color]} transition-transform group-hover:scale-110">
        {icon}
      </div>
      <div class="flex-1 min-w-0">
        <p class="text-xs font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wider">{title}</p>
        <p class="text-xl font-bold text-slate-800 dark:text-white mt-0.5 truncate">{value}</p>
      </div>
      <svg class="w-4 h-4 text-slate-300 group-hover:text-indigo-500 transition-all group-hover:translate-x-1" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
      </svg>
    </div>
  </a>
{:else}
  <div class="p-5 rounded-2xl bg-white dark:bg-slate-800/80 ring-1 ring-slate-200/50 dark:ring-slate-700/50">
    <div class="flex items-center gap-4">
      <div class="w-11 h-11 rounded-xl bg-gradient-to-br {gradients[color]} flex items-center justify-center text-white text-lg shadow-lg {shadowColors[color]}">
        {icon}
      </div>
      <div class="min-w-0">
        <p class="text-xs font-medium text-slate-400 dark:text-slate-500 uppercase tracking-wider">{title}</p>
        <p class="text-xl font-bold text-slate-800 dark:text-white mt-0.5 truncate">{value}</p>
      </div>
    </div>
  </div>
{/if}
