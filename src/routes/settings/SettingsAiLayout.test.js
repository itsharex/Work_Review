import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('AI 设置中的 API 密钥输入应支持显示与隐藏切换', async () => {
  const source = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /let showApiKey = false;/);
  assert.match(source, /\{#if showApiKey\}/);
  assert.match(source, /type="text"/);
  assert.match(source, /type="password"/);
  assert.match(source, /aria-label=\{showApiKey \? '隐藏 API 密钥' : '显示 API 密钥'\}/);
});

test('日报导出目录应从 AI 设置移到存储设置', async () => {
  const aiSource = await readFile(
    new URL('./components/SettingsAI.svelte', import.meta.url),
    'utf8'
  );
  const storageSource = await readFile(
    new URL('./components/SettingsStorage.svelte', import.meta.url),
    'utf8'
  );

  assert.doesNotMatch(aiSource, /日报 Markdown 导出目录/);
  assert.match(storageSource, /日报 Markdown 导出目录/);
  assert.match(storageSource, /pickDailyReportExportDir/);
  assert.match(storageSource, /自动导出 YYYY-MM-DD\.md/);
  assert.match(storageSource, /设置日报 Markdown 默认下载位置。/);
  assert.match(storageSource, /h3 class="settings-card-title">截图与保留</);
  assert.match(storageSource, /h3 class="settings-card-title">日报导出</);
  assert.match(storageSource, /h3 class="settings-card-title">数据目录与清理</);
  assert.match(storageSource, /整桌面拼接截图/);
  assert.match(storageSource, /把所有显示器内容拼成一张完整截图/);
});
