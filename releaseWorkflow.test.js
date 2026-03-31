import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';

test('Release workflow 应在构建前执行测试并使用 npm ci', () => {
  const source = readFileSync(new URL('./.github/workflows/release.yml', import.meta.url), 'utf8');

  assert.match(source, /run:\s*npm ci/);
  assert.match(source, /name:\s*Run frontend tests/);
  assert.match(source, /run:\s*node --test/);
  assert.match(source, /name:\s*Build frontend assets for Rust tests/);
  assert.match(source, /run:\s*npm run build/);
  assert.match(source, /name:\s*Run Rust tests/);
  assert.match(source, /run:\s*cargo test --manifest-path src-tauri\/Cargo\.toml/);

  const frontendIndex = source.indexOf('name: Run frontend tests');
  const frontendBuildIndex = source.indexOf('name: Build frontend assets for Rust tests');
  const rustIndex = source.indexOf('name: Run Rust tests');
  const buildIndex = source.indexOf('name: Build application');

  assert.notEqual(frontendIndex, -1);
  assert.notEqual(frontendBuildIndex, -1);
  assert.notEqual(rustIndex, -1);
  assert.notEqual(buildIndex, -1);
  assert.ok(frontendIndex < buildIndex, '前端测试必须先于构建执行');
  assert.ok(frontendBuildIndex < rustIndex, 'Rust 测试前必须先生成 frontendDist');
  assert.ok(rustIndex < buildIndex, 'Rust 测试必须先于构建执行');
});
