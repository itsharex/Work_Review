import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('概览页面的浏览器详情列表应格式化显示 URL', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(
    source,
    /formatBrowserUrlForDisplay\(url\.url\)/,
    '概览页的浏览器详情列表应对原始 URL 做可读化处理'
  );
});

test('概览页面应支持在网站访问弹层中直接修改域名语义分类并回填历史', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /invoke\('set_domain_semantic_rule'/);
  assert.match(source, /translateSemanticCategoryLabel/);
  assert.match(source, /overview\.changeDomainCategoryMessage/);
  assert.match(source, /overview\.selectCategory/);
  assert.match(source, /overview\.currentCategory/);
  assert.match(source, /<select/);
  assert.match(source, /semanticCategoryOptions/);
  assert.match(source, /编码开发/);
  assert.match(source, /资料阅读/);
  assert.match(source, /休息娱乐/);
});

test('概览页面应展示按小时活跃度柱状图', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /ActivityHourlyChart/);
  assert.match(source, /overview\.hourlyActivity/);
  assert.match(source, /stats\.hourly_activity_distribution/);
  assert.match(source, /hourlyChartDistributionTitle/);
  assert.match(source, /hourlyChartDistributionSubtitleKey/);
  assert.match(source, /hourlyChart\.distributionTitleToday/);
  assert.match(source, /hourlyChart\.distributionTitleWeek/);
  assert.match(source, /hourlyChart\.distributionTitleRange/);
  assert.ok(
    source.indexOf('overview.appUsage') < source.indexOf('overview.hourlyActivity'),
    '按小时活跃度应位于应用使用模块下方'
  );
});

test('概览页面在不可见时应暂停时钟与定时刷新', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /document\.addEventListener\('visibilitychange'/);
  assert.match(
    source,
    /if\s*\(document\.hidden\)[\s\S]*clearInterval\(clockInterval\)[\s\S]*clearInterval\(refreshInterval\)/
  );
  assert.match(
    source,
    /else\s*\{[\s\S]*clockInterval\s*=\s*setInterval[\s\S]*refreshInterval\s*=\s*setInterval/
  );
  assert.match(source, /document\.removeEventListener\('visibilitychange'/);
});

test('概览页面应支持今日、指定日期与本周三种时间视角切换', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /invoke\('get_overview_stats'/);
  assert.match(source, /overview\.modeToday/);
  assert.match(source, /overview\.modeDate/);
  assert.match(source, /overview\.modeWeek/);
  assert.match(source, /\{#if overviewMode === 'date'\}/);
  assert.match(source, /selectedDateFrom/);
  assert.match(source, /selectedDateTo/);
  assert.match(source, /overviewDateInputFrom/);
  assert.match(source, /overviewDateInputTo/);
  assert.match(source, /editingOverviewDateFrom/);
  assert.match(source, /editingOverviewDateTo/);
  assert.match(source, /stepOverviewDateBoundary/);
  assert.match(source, /commitOverviewDateInput/);
  assert.match(source, /inputmode="numeric"/);
  assert.match(source, /type="text"/);
  assert.match(source, /on:focus=\{\(\) => \{ editingOverviewDateFrom = true; \}\}/);
  assert.match(source, /on:focus=\{\(\) => \{ editingOverviewDateTo = true; \}\}/);
  assert.match(source, /dateFrom: overviewMode === 'date'/);
  assert.match(source, /dateTo: overviewMode === 'date'/);
  const todayIndex = source.indexOf("setOverviewMode('today')");
  const weekIndex = source.indexOf("setOverviewMode('week')");
  const dateIndex = source.indexOf("setOverviewMode('date')");
  assert.ok(
    todayIndex < weekIndex && weekIndex < dateIndex,
    '顶部视角顺序应为 今日 -> 本周 -> 指定日期'
  );
});

test('概览页面的指定日期选择框应跟随当前语言并使用紧凑控件样式', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /overview-date-bar/);
  assert.match(source, /overview-date-input/);
  assert.match(source, /overview-date-field/);
  assert.match(source, /stepOverviewDateBoundary\('start'/);
  assert.match(source, /stepOverviewDateBoundary\('end'/);
  assert.doesNotMatch(source, /LocalizedDatePicker/);
});

test('概览卡片标题应随今日、单日、范围和本周视角切换', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /overviewTotalActivityTitle/);
  assert.match(source, /overviewWorkDurationTitle/);
  assert.match(source, /overview\.totalActivityToday/);
  assert.match(source, /overview\.totalActivityDate/);
  assert.match(source, /overview\.totalActivityRange/);
  assert.match(source, /overview\.totalActivityWeek/);
  assert.match(source, /overview\.workDurationToday/);
  assert.match(source, /overview\.workDurationDate/);
  assert.match(source, /overview\.workDurationRange/);
  assert.match(source, /overview\.workDurationWeek/);
});

test('概览页的应用使用与按小时活跃度应支持视图切换并记忆用户偏好', async () => {
  const source = await readFile(new URL('./Overview.svelte', import.meta.url), 'utf8');

  assert.match(source, /appUsageViewMode/);
  assert.match(source, /hourlyActivityViewMode/);
  assert.match(source, /APP_USAGE_VIEW_MODE_KEY = 'overview\.appUsage\.viewMode'/);
  assert.match(source, /HOURLY_ACTIVITY_VIEW_MODE_KEY = 'overview\.hourlyActivity\.viewMode'/);
  assert.match(source, /overview\.appUsageBar/);
  assert.match(source, /overview\.appUsageColumn/);
  assert.match(source, /overview\.hourlyActivityBar/);
  assert.match(source, /overview\.hourlyActivityColumn/);
  assert.match(source, /readStoredOverviewViewMode/);
  assert.match(source, /persistOverviewViewMode/);
  assert.match(source, /localStorage\.getItem\(key\)/);
  assert.match(source, /localStorage\.setItem\(key, value\)/);
});

test('概览统计命令应注册为 get_overview_stats', async () => {
  const source = await readFile(new URL('../../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(source, /commands::get_overview_stats/);
});
