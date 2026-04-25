import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('设置页应提供节点 Beta 标签并在设置工作台内渲染节点组件', async () => {
  const source = await readFile(new URL('./Settings.svelte', import.meta.url), 'utf8');

  assert.match(source, /import SettingsNodeGateway from '\.\/components\/SettingsNodeGateway\.svelte'/);
  assert.match(source, /id:\s*'node'/);
  assert.match(source, /labelKey:\s*'settings\.tabs\.node'/);
  assert.match(source, /beta:\s*true/);
  assert.match(source, /t\('settings\.tabs\.beta'\)/);
  assert.match(source, /activeTab === 'node'/);
  assert.match(source, /<SettingsNodeGateway bind:config/);

  const storageTabIndex = source.indexOf("id: 'storage'");
  const nodeTabIndex = source.indexOf("id: 'node'");
  assert.notEqual(storageTabIndex, -1);
  assert.notEqual(nodeTabIndex, -1);
  assert.ok(nodeTabIndex > storageTabIndex, '节点标签应位于存储标签之后');
});

test('节点设置组件应复用设置页配置对象并读取节点与本地 API 状态', async () => {
  const source = await readFile(
    new URL('./components/SettingsNodeGateway.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /export let config/);
  assert.match(source, /invoke\('get_node_gateway_status'\)/);
  assert.match(source, /invoke\('get_localhost_api_status'\)/);
  assert.match(source, /invoke\('save_config', \{ config \}\)/);
  assert.match(source, /node-gateway-settings-shell/);
  assert.match(source, /settingsCardBeta/);
});

test('节点设置组件应提供注册与心跳动作，而不是只能停留在静态配置态', async () => {
  const source = await readFile(
    new URL('./components/SettingsNodeGateway.svelte', import.meta.url),
    'utf8'
  );

  assert.match(source, /invoke\('register_node_gateway'\)/);
  assert.match(source, /invoke\('send_node_gateway_heartbeat'\)/);
  assert.match(source, /nodeGatewayPage\.registerDevice/);
  assert.match(source, /nodeGatewayPage\.sendHeartbeat/);
});
