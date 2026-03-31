import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('Cargo.toml 应声明 cargo-clippy 兼容 feature 以避免 objc 宏触发 check-cfg 误报', () => {
  const source = readFileSync(new URL('./src-tauri/Cargo.toml', import.meta.url), 'utf8');

  assert.match(
    source,
    /\[features\][\s\S]*\bcargo-clippy\s*=\s*\[\s*\]/,
    '需要显式声明 cargo-clippy feature，兼容旧 objc 宏里的 cfg(feature = "cargo-clippy")'
  );
});
