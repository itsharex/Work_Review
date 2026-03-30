import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('日报生成应在网站访问部分体现域名语义分类', async () => {
  const [summarySource, localSource] = await Promise.all([
    readFile(new URL('../src-tauri/src/analysis/summary.rs', import.meta.url), 'utf8'),
    readFile(new URL('../src-tauri/src/analysis/local.rs', import.meta.url), 'utf8'),
  ]);

  assert.match(summarySource, /domain\.semantic_category/);
  assert.match(localSource, /domain\.semantic_category/);
});
