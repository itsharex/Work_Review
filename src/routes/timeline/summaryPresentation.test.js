import test from 'node:test';
import assert from 'node:assert/strict';

import {
  getMainApps,
  getPrimarySummary,
  getSecondarySummary,
  getSummaryRhythmMeta,
} from './summaryPresentation.js';

test('时段摘要应拆出主摘要和副摘要', () => {
  const source = '上午主要处理日报生成逻辑，补齐了状态提示，并完成了回归验证。下午继续优化样式细节。';

  assert.equal(getPrimarySummary(source), '上午主要处理日报生成逻辑，补齐了状态提示');
  assert.equal(getSecondarySummary(source), '并完成了回归验证。');
});

test('时段摘要应按时长生成节奏标签', () => {
  assert.deepEqual(getSummaryRhythmMeta(55 * 60), {
    tone: 'deep',
    label: '深度推进',
  });
  assert.deepEqual(getSummaryRhythmMeta(28 * 60), {
    tone: 'steady',
    label: '持续推进',
  });
  assert.deepEqual(getSummaryRhythmMeta(8 * 60), {
    tone: 'light',
    label: '轻量切换',
  });
});

test('主应用列表应限制数量并过滤空项', () => {
  assert.deepEqual(
    getMainApps('Cursor, Google Chrome, Slack, Terminal, Notes'),
    ['Cursor', 'Google Chrome', 'Slack', 'Terminal']
  );
});
