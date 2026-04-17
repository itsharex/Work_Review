import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

test('Linux 后端应识别 X11 与 Wayland 会话类型', async () => {
  const source = await readFile(new URL('./src/linux_session.rs', import.meta.url), 'utf8');

  assert.match(source, /pub enum LinuxDesktopSession/);
  assert.match(source, /pub enum LinuxDesktopEnvironment/);
  assert.match(source, /Wayland/);
  assert.match(source, /X11/);
  assert.match(source, /Gnome/);
  assert.match(source, /XDG_SESSION_TYPE/);
  assert.match(source, /WAYLAND_DISPLAY/);
  assert.match(source, /DISPLAY/);
  assert.match(source, /XDG_CURRENT_DESKTOP/);
  assert.match(source, /DESKTOP_SESSION/);
  assert.match(source, /current_linux_desktop_environment/);
});

test('Linux 截图应按会话类型分流，Wayland 优先尝试原生工具', async () => {
  const source = await readFile(new URL('./src/screenshot.rs', import.meta.url), 'utf8');

  assert.match(source, /LinuxDesktopSession::Wayland/);
  assert.match(source, /capture_linux_wayland/);
  assert.match(source, /grim/);
  assert.match(source, /gnome-screenshot/);
  assert.match(source, /spectacle/);
});

test('手动截图在 Linux Wayland 下不应强依赖活动窗口检测', async () => {
  const source = await readFile(new URL('./src/commands.rs', import.meta.url), 'utf8');

  assert.match(source, /let active_window = crate::monitor::get_active_window\(\)\.ok\(\);/);
  assert.match(source, /capture_for_window\(active_window\.as_ref\(\)\)/);
  assert.match(source, /unwrap_or_else\(\|\| "Wayland Session"\.to_string\(\)\)/);
});

test('Linux 活动窗口检测应为 GNOME Wayland 单独提供 provider 分流', async () => {
  const source = await readFile(new URL('./src/monitor.rs', import.meta.url), 'utf8');

  assert.match(source, /current_linux_active_window_provider/);
  assert.match(source, /get_active_window_linux_x11/);
  assert.match(source, /get_active_window_linux_wayland_gnome/);
  assert.match(source, /get_active_window_linux_wayland_kde/);
  assert.match(source, /get_active_window_linux_wayland_sway/);
  assert.match(source, /get_active_window_linux_wayland_hyprland/);
  assert.match(source, /LinuxDesktopEnvironment::Gnome/);
  assert.match(source, /org\.gnome\.shell\.extensions\.FocusedWindow\.Get/);
  assert.match(source, /gdbus/);
  assert.match(source, /kdotool/);
  assert.match(source, /swaymsg/);
  assert.match(source, /hyprctl/);
  assert.match(source, /WindowBounds/);
  assert.match(source, /parse_xdotool_geometry_shell_output/);
  assert.match(source, /parse_window_bounds_from_json/);
  assert.match(source, /resolve_browser_url_for_window_linux/);
  assert.match(source, /firefox_family_session_store_url/);
});

test('GNOME Wayland 桌宠联动应内置专用扩展并通过 D-Bus 暴露指针信息', async () => {
  const inputSource = await readFile(new URL('./src/avatar_input.rs', import.meta.url), 'utf8');
  const metadata = await readFile(
    new URL('../scripts/gnome-shell/work-review-avatar-input@workreview.app/metadata.json', import.meta.url),
    'utf8'
  );
  const extensionSource = await readFile(
    new URL('../scripts/gnome-shell/work-review-avatar-input@workreview.app/extension.js', import.meta.url),
    'utf8'
  );

  assert.match(inputSource, /WorkReviewAvatarInput\.GetPointer/);
  assert.match(inputSource, /gnome-shell-dbus/);
  assert.match(metadata, /work-review-avatar-input@workreview\.app/);
  assert.match(extensionSource, /global\.get_pointer\(\)/);
  assert.match(extensionSource, /org\.gnome\.shell\.extensions\.WorkReviewAvatarInput/);
  assert.match(extensionSource, /mouseGroup/);
});
