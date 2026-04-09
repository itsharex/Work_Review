import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

import { AVATAR_OUTLINE_LAYOUT, getAvatarOutline } from './avatarOutline.js';
import { formatBubbleMessage } from './bubbleMessage.js';

test('桌宠应保持纯轮廓布局，不再包含桌面或方框结构', () => {
  assert.equal(AVATAR_OUTLINE_LAYOUT.showDesk, false);
  assert.equal(AVATAR_OUTLINE_LAYOUT.showFrame, false);
  assert.equal(AVATAR_OUTLINE_LAYOUT.viewBox, '50 40 128 138');
  assert.equal(AVATAR_OUTLINE_LAYOUT.figureClass, 'relative h-full w-full avatar-float');
});

test('桌宠应保留分段轮廓结构，只移除额外方框', () => {
  const outline = getAvatarOutline();

  assert.match(outline.headPath, /L74 50/);
  assert.match(outline.bodyPath, /L121 160/);
  assert.equal(typeof outline.bellyPath, 'undefined');
  assert.match(outline.tailPath, /Z$/);
  assert.match(outline.leftPawPath, /59 154/);
  assert.match(outline.rightPawPath, /141 154/);
  assert.equal(outline.leftEarInnerPath, 'M74 74 L80 57 L88 74');
  assert.equal(outline.rightEarInnerPath, 'M113 74 L120 57 L126 73');
});

test('桌宠路径不应复用全局 outline 类名，避免 SVG 包围框泄漏', () => {
  const source = readFileSync(new URL('./AvatarCanvas.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');
  const engineSource = readFileSync(new URL('../../../../src-tauri/src/avatar_engine.rs', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /class="outline"/);
  assert.match(source, /avatar-stroke/);
  assert.match(source, /avatar-fill/);
  assert.match(source, /tail-detail/);
  assert.match(source, /ear-detail/);
  assert.match(source, /cheek-detail/);
  assert.match(source, /avatar-shell/);
  assert.match(source, /--avatar-shell-opacity/);
  assert.match(source, /class="avatar-hit avatar-fill avatar-stroke tail-detail"/);
  assert.match(source, /pointer-events:\s*visiblePainted/);
  assert.match(source, /getAvatarIdleMotionMeta/);
  assert.match(source, /state\.contextLabel/);
  assert.match(source, /motionBeat/);
  assert.match(source, /transitionClass/);
  assert.match(source, /idle-breathe/);
  assert.match(source, /transition-focus-shift/);
  assert.doesNotMatch(source, /radial-gradient\(circle/);
  assert.doesNotMatch(source, /accent-line/);
  assert.doesNotMatch(source, /thought/);
  assert.doesNotMatch(windowSource, /aria-label="打开主界面"/);
  assert.match(windowSource, /on:avatarpointerdown=\{startAvatarDrag\}/);
  assert.match(windowSource, /on:avataractivate=\{openMainWindow\}/);
  assert.match(windowSource, /getAvatarTransitionMeta/);
  assert.match(windowSource, /getAvatarMotionStepDelay/);
  assert.match(windowSource, /motionBeat = \(motionBeat \+ 1\) % 96/);
  assert.match(windowSource, /scheduleNextMotionStep/);
  assert.match(windowSource, /transitionClass = transition\.className/);
  assert.match(windowSource, /getAvatarStateBubble/);
  assert.match(windowSource, /showBubble\(stateBubble\)/);
  assert.match(
    windowSource,
    /nextState\.mode !== state\.mode\s*\|\|\s*nextState\.contextLabel !== state\.contextLabel/
  );
  assert.match(engineSource, /const AVATAR_SCALE_DEFAULT: f64 = 0\.9;/);
  assert.match(engineSource, /const AVATAR_WINDOW_BASE_WIDTH: f64 = 276\.0;/);
  assert.match(engineSource, /const AVATAR_WINDOW_BASE_HEIGHT: f64 = 170\.0;/);
  assert.match(engineSource, /const AVATAR_WINDOW_MARGIN: f64 = 8\.0;/);
});

test('状态气泡应悬浮在猫头上方，采用紧凑气泡而不是横条', () => {
  const source = readFileSync(new URL('./AvatarPopover.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /Array\.from\(bubble\.message\.replace\(\/\\s\+\/g, ''\)\)/);
  assert.match(source, /style="right: 18%; top: 4%;"/);
  assert.doesNotMatch(source, /writing-mode: vertical-rl/);
  assert.match(source, /width: fit-content/);
  assert.match(source, /max-width: min\(58vw, 188px\)/);
  assert.match(source, /min-width: 0/);
  assert.match(source, /display: inline-block;/);
  assert.match(source, /bg-\[linear-gradient\(180deg,\s*rgba\(15,23,42,0\.97\),\s*rgba\(30,41,59,0\.93\)\)\] text-slate-100/);
  assert.match(source, /bg-\[linear-gradient\(180deg,\s*rgba\(6,78,59,0\.97\),\s*rgba\(6,95,70,0\.94\)\)\] text-emerald-50/);
  assert.match(source, /shadow-\[0_10px_18px_rgba\(15,23,42,0\.18\),0_24px_44px_rgba\(15,23,42,0\.26\)\]/);
  assert.match(source, /absolute inset-\[1px\] rounded-\[21px\] border border-white\/10/);
  assert.match(source, /text-sm font-semibold leading-\[1\.35\] tracking-\[0\.01em\]/);
  assert.match(source, /rounded-\[22px\]/);
  assert.match(source, /left-\[18px\] top-\[calc\(100%-4px\)\]/);
  assert.match(source, /h-\[7px\] w-\[7px\] rounded-full/);
  assert.match(source, /h-\[11px\] w-\[11px\] rounded-full/);
  assert.match(windowSource, /class="h-full w-\[54%\]"/);
  assert.doesNotMatch(source, /-translate-x-1\/2/);
});

test('休息提醒气泡应支持常驻显示和手动关闭', () => {
  const source = readFileSync(new URL('./AvatarPopover.svelte', import.meta.url), 'utf8');
  const windowSource = readFileSync(new URL('../../../routes/avatar/AvatarWindow.svelte', import.meta.url), 'utf8');

  assert.match(source, /export let onClose = \(\) => \{\};/);
  assert.match(source, /bubble\?\.persistent/);
  assert.match(source, /class="absolute inset-0 z-20 overflow-visible pointer-events-none"/);
  assert.match(source, /class="pointer-events-auto relative rounded-\[22px\]/);
  assert.match(source, /<button[\s\S]*type="button"[\s\S]*on:click=\{onClose\}/);
  assert.match(source, /aria-label="关闭提醒"/);
  assert.match(windowSource, /<AvatarPopover \{bubble\} onClose=\{dismissBubble\} \/>/);
  assert.match(windowSource, /if \(!payload\?\.persistent\)/);
});

test('英文桌宠气泡文案应保留单词间空格', () => {
  assert.equal(
    formatBubbleMessage('Time for a break. Stand up and stretch a bit.'),
    'Time for a break. Stand up and stretch a bit.'
  );
});
