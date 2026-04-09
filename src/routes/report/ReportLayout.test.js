import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('日报页头部应使用独立布局以适配英文长标题与日期信息', async () => {
  const [reportSource, appCssSource] = await Promise.all([
    readFile(new URL('./Report.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../../app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(reportSource, /class="report-hero"/);
  assert.match(reportSource, /class="report-hero-main"/);
  assert.match(reportSource, /class="report-hero-meta"/);
  assert.match(reportSource, /class="report-hero-actions"/);
  assert.match(reportSource, /report-hero-date-row/);
  assert.match(reportSource, /report-hero-status-row/);
  assert.match(reportSource, /report-hero-summary-line/);
  assert.match(reportSource, /report-hero-summary-value/);
  assert.match(reportSource, /report-hero-config-note/);
  assert.match(reportSource, /report-hero-mode-note/);
  assert.doesNotMatch(reportSource, /report-hero-status-label/);
  assert.doesNotMatch(reportSource, /report-hero-model-chip/);
  assert.doesNotMatch(reportSource, /<div class="page-header">/);

  assert.match(appCssSource, /\.report-hero\b/);
  assert.match(appCssSource, /\.report-hero-main\b/);
  assert.match(appCssSource, /\.report-hero-meta\b/);
  assert.match(appCssSource, /\.report-hero-actions\b/);
  assert.match(appCssSource, /\.report-hero-date-row\b/);
  assert.match(appCssSource, /\.report-hero-status-row\b/);
  assert.match(appCssSource, /\.report-hero-summary-line\b/);
  assert.match(appCssSource, /\.report-hero-summary-value\b/);
  assert.match(appCssSource, /\.report-hero-config-note\b/);
  assert.match(appCssSource, /\.report-hero-mode-note\b/);
  assert.doesNotMatch(appCssSource, /\.report-hero-status-label\b/);
});

test('昨日日报提示条应为独立动作区提供响应式布局，避免生成中按钮被压扁', async () => {
  const [reportSource, appCssSource] = await Promise.all([
    readFile(new URL('./Report.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../../app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(reportSource, /report-fallback-banner/);
  assert.match(reportSource, /report-fallback-copy/);
  assert.match(reportSource, /report-fallback-action/);
  assert.match(reportSource, /report-fallback-button/);

  assert.match(appCssSource, /\.report-fallback-banner\b/);
  assert.match(appCssSource, /\.report-fallback-copy\b/);
  assert.match(appCssSource, /\.report-fallback-action\b/);
  assert.match(appCssSource, /\.report-fallback-button\b/);
});

test('日报页纸面容器应复用统一 editorial surface，而不是额外偏黄底色', async () => {
  const appCssSource = await readFile(new URL('../../app.css', import.meta.url), 'utf8');

  assert.match(appCssSource, /\.report-sheet-controls\s*\{[\s\S]*background:\s*var\(--editorial-surface-subtle\)/);
  assert.match(appCssSource, /\.report-article-card\s*\{[\s\S]*background:\s*var\(--editorial-surface-featured\)/);
  assert.match(appCssSource, /\.report-sheet::before\s*\{[\s\S]*rgba\(99,\s*102,\s*241,\s*0\.014\)/);
});
