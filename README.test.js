import test from 'node:test';
import assert from 'node:assert/strict';
import { access, readFile } from 'node:fs/promises';

test('README 应提供中英文切换入口并包含英文文档', async () => {
  const source = await readFile(new URL('./README.md', import.meta.url), 'utf8');

  assert.match(
    source,
    /href="\.\/*README\.en\.md"[^>]*>English<\/a>/,
    'README 顶部应提供 English 切换链接'
  );
  assert.match(
    source,
    /href="\.\/*README\.md"[^>]*>中文<\/a>/,
    'README 顶部应保留中文入口，形成双语切换'
  );

  await access(new URL('./README.en.md', import.meta.url));
});
