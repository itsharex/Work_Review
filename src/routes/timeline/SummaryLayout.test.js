import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('时段摘要页应渲染编辑部阶段带状布局', async () => {
  const source = await readFile(new URL('./Summary.svelte', import.meta.url), 'utf8');

  assert.match(source, /summary-editorial-shell/);
  assert.match(source, /summary-band/);
  assert.match(source, /summary-band-card/);
  assert.match(source, /summary-app-tags/);
});

test('时段摘要页应提取一句主摘要并替换旧的 bullet 列表模式', async () => {
  const source = await readFile(new URL('./Summary.svelte', import.meta.url), 'utf8');

  assert.match(source, /getPrimarySummary/);
  assert.match(source, /getMainApps/);
  assert.match(source, /summary-primary-copy/);
  assert.doesNotMatch(source, /function formatSummary/);
});

test('时段摘要页应增加副摘要与节奏信息层，而不是只显示一句话和应用标签', async () => {
  const source = await readFile(new URL('./Summary.svelte', import.meta.url), 'utf8');

  assert.match(source, /summary-secondary-copy/);
  assert.match(source, /summary-meta-row/);
  assert.match(source, /summary-rhythm-chip/);
  assert.match(source, /summary-app-count/);
});
