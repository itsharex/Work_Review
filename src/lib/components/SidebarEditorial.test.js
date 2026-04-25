import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('侧边栏应提供编辑部导航框架', async () => {
  const [source, appCssSource] = await Promise.all([
    readFile(new URL('./Sidebar.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../../app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(source, /sidebar-editorial-shell/);
  assert.match(source, /sidebar-nav-section/);
  assert.match(source, /sidebar-brand-panel/);
  assert.match(source, /sidebar-status-panel/);
  assert.match(source, /sidebar-toolbelt/);
  assert.match(source, /sidebar-brand-line/);
  assert.match(source, /sidebar-brand-separator/);
  assert.doesNotMatch(source, /sidebar-brand-chip/);
  assert.doesNotMatch(source, /sidebar-nav-index/);
  assert.match(appCssSource, /\.sidebar-editorial-shell\s*\{[\s\S]*background:\s*transparent;/);
  assert.match(appCssSource, /\.sidebar-brand-panel\s*\{[\s\S]*background:\s*transparent;/);
  assert.match(appCssSource, /\.sidebar-brand-panel\s*\{[\s\S]*border:\s*none;/);
  assert.match(appCssSource, /\.sidebar-status-panel\s*\{[\s\S]*background:\s*transparent;/);
  assert.match(appCssSource, /\.sidebar-status-panel\s*\{[\s\S]*box-shadow:\s*none;/);
  assert.match(appCssSource, /\.sidebar-nav-section\s*\{[\s\S]*background:\s*transparent;/);
  assert.match(appCssSource, /\.sidebar-nav-section\s*\{[\s\S]*border:\s*none;/);
  assert.match(appCssSource, /\.sidebar-toolbelt\s*\{[\s\S]*background:\s*transparent;/);
  assert.match(appCssSource, /\.sidebar-toolbelt\s*\{[\s\S]*border:\s*none;/);
});

test('侧边栏品牌副标题在英文下应允许换行而不是被硬裁切', async () => {
  const [source, appCssSource] = await Promise.all([
    readFile(new URL('./Sidebar.svelte', import.meta.url), 'utf8'),
    readFile(new URL('../../app.css', import.meta.url), 'utf8'),
  ]);

  assert.match(source, /sidebar-brand-line/);
  assert.match(source, /sidebar-brand-segment/);
  assert.match(appCssSource, /\.sidebar-brand-line\b[\s\S]*flex-wrap:/);
  assert.match(appCssSource, /\.sidebar-brand-segment\b/);
});

test('侧边栏激活态高亮条应位于图标区外侧，避免与导航图标重叠', async () => {
  const appCssSource = await readFile(new URL('../../app.css', import.meta.url), 'utf8');

  assert.match(appCssSource, /\.sidebar-nav-rail\s*\{[\s\S]*left:\s*0\.55rem/);
  assert.doesNotMatch(appCssSource, /\.sidebar-nav-rail\s*\{[\s\S]*left:\s*0\.95rem/);
});

test('侧边栏不应继续提供独立的设备节点入口，节点能力应收回设置页 Beta 标签', async () => {
  const source = await readFile(new URL('./Sidebar.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /path:\s*'\/node'/);
  assert.doesNotMatch(source, /labelKey:\s*'sidebar\.nav\.node'/);
  assert.doesNotMatch(source, /item\.icon === 'node'/);
});
