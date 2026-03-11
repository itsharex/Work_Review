// 应用图标全局缓存（模块级单例，跨页面导航不丢失）
// 图标通过 Tauri 后端获取并转为 base64，此缓存避免重复 invoke 调用
import { writable } from 'svelte/store';

// 模块级缓存对象，不随组件销毁而丢失
const _iconCache = {};
const _pendingRequests = {};

// 响应式 store，通知 Svelte 更新 UI
export const appIconStore = writable({});

// 加载指定应用的图标
export async function loadAppIcon(appName, invoke) {
    // 已缓存（成功或失败），直接返回
    if (_iconCache[appName] !== undefined) return;

    // 避免同一应用并发请求
    if (_pendingRequests[appName]) return;
    _pendingRequests[appName] = true;

    try {
        const base64 = await invoke('get_app_icon', { appName });
        if (base64 && base64.length > 100) {
            _iconCache[appName] = base64;
        } else {
            _iconCache[appName] = null;
        }
    } catch {
        _iconCache[appName] = null;
    } finally {
        delete _pendingRequests[appName];
        // 更新 store 触发 UI 重新渲染
        appIconStore.set({ ..._iconCache });
    }
}

// 批量预加载
export function preloadAppIcons(names, invoke) {
    names.forEach(name => loadAppIcon(name, invoke));
}

// 获取已缓存的图标（同步）
export function getIcon(appName) {
    return _iconCache[appName] || null;
}
