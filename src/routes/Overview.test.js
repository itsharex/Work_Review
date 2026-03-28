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
