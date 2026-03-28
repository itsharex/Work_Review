import test from 'node:test';
import assert from 'node:assert/strict';

import { formatBrowserUrlForDisplay } from './browserUrl.js';

test('应将 URL 中的中文查询词解码为可读文本', () => {
  const rawUrl =
    'https://www.google.com.hk/search?q=%E5%A4%A7%E6%B8%A1%E5%8F%A3&client=firefox-b-d';

  assert.equal(
    formatBrowserUrlForDisplay(rawUrl),
    'https://www.google.com.hk/search?q=大渡口&client=firefox-b-d'
  );
});

test('应保留会改变 URL 结构语义的 ASCII 编码字符', () => {
  const rawUrl = 'https://example.com/search?q=a%26b&name=%E5%BC%A0%E4%B8%89';

  assert.equal(
    formatBrowserUrlForDisplay(rawUrl),
    'https://example.com/search?q=a%26b&name=张三'
  );
});

test('无效编码时应保留坏片段并尽量解码可读部分', () => {
  const rawUrl = 'https://example.com/search?q=%E5%A4%A7%ZZ';

  assert.equal(formatBrowserUrlForDisplay(rawUrl), 'https://example.com/search?q=大%ZZ');
});
