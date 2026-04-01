import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('按小时活跃度图表应支持点击柱子查看所选时段时长', async () => {
  const source = await readFile(new URL('./ActivityHourlyChart.svelte', import.meta.url), 'utf8');

  assert.match(source, /let selectedHour = null/);
  assert.match(source, /function selectHour\(hour\)/);
  assert.match(source, /selectedBucket = buckets\.find\(\(bucket\) => bucket\.hour === selectedHour\) \|\| null/);
  assert.match(source, /aria-pressed=\{selectedHour === bucket\.hour\}/);
  assert.match(source, /on:click=\{\(\) => selectHour\(bucket\.hour\)\}/);
  assert.match(source, /hourlyChart\.selectedHour/);
  assert.match(source, /hourlyChart\.selectedHourHint/);
});

test('按小时活跃度图表文案应为三种语言补齐点击查看时段信息', async () => {
  const source = await readFile(new URL('../i18n/index.js', import.meta.url), 'utf8');

  assert.equal((source.match(/selectedHour:/g) || []).length, 3);
  assert.equal((source.match(/selectedHourHint:/g) || []).length, 3);
});
