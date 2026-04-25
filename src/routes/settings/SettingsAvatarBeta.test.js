import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('桌面化身 Beta 应显示在外层标签栏而不是内容卡内部', async () => {
  const [settingsSource, appearanceSource] = await Promise.all([
    readFile(new URL('./Settings.svelte', import.meta.url), 'utf8'),
    readFile(new URL('./components/SettingsAppearance.svelte', import.meta.url), 'utf8'),
  ]);

  assert.match(settingsSource, /id:\s*'avatar'[^\n]*beta:\s*true/);
  assert.doesNotMatch(appearanceSource, />\s*Beta\s*</);
});
