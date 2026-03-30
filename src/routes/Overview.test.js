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
  assert.match(source, /按域名生效，并回填该站点的历史记录/);
  assert.match(source, /修改分类/);
  assert.match(source, /当前分类/);
});
