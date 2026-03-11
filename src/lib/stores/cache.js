// 全局数据缓存 Store
// 使用 Svelte writable store 缓存各页面数据，避免重复请求

import { writable, derived } from 'svelte/store';

// 获取本地日期
function getLocalDateString() {
  const now = new Date();
  const year = now.getFullYear();
  const month = String(now.getMonth() + 1).padStart(2, '0');
  const day = String(now.getDate()).padStart(2, '0');
  return `${year}-${month}-${day}`;
}

// 缓存数据结构
function createCache() {
  const { subscribe, set, update } = writable({
    // 概览数据（包含日期标记用于跨天检测）
    overview: {
      data: null,
      timestamp: 0,
      loading: false,
      date: null,  // 记录数据对应的日期
    },
    // 时间线数据（按日期缓存）
    timeline: {},
    // 日报数据（按日期缓存）
    reports: {},
    // 小时摘要（按日期缓存）
    hourlySummaries: {},
    // 配置
    config: null,
  });

  // 区分不同数据类型的 TTL（毫秒）
  const CACHE_TTL = {
    overview: 15000,    // 概览：15秒（高频变化）
    timeline: 30000,    // 时间线：30秒
    reports: 300000,    // 日报：5分钟（低频变化）
    default: 30000,     // 默认：30秒
  };

  return {
    subscribe,

    // 检查缓存是否有效（包含跨天检测）
    isValid: (cache, key = null) => {
      if (!cache) return false;
      const data = key ? cache[key] : cache;
      if (!data || !data.timestamp) return false;

      // 概览数据跨天检测：日期不匹配则缓存失效
      if (key === 'overview' && data.date && data.date !== getLocalDateString()) {
        return false;
      }

      // 根据数据类型选择 TTL
      const ttl = key ? (CACHE_TTL[key] || CACHE_TTL.default) : CACHE_TTL.default;
      return Date.now() - data.timestamp < ttl;
    },

    // 设置概览数据（同时记录日期用于跨天检测）
    setOverview: (data) => update(c => ({
      ...c,
      overview: { data, timestamp: Date.now(), loading: false, date: getLocalDateString() }
    })),

    // 设置时间线数据
    setTimeline: (date, data, summaries) => update(c => ({
      ...c,
      timeline: { ...c.timeline, [date]: { data, summaries, timestamp: Date.now() } }
    })),

    // 设置日报数据
    setReport: (date, data) => update(c => ({
      ...c,
      reports: { ...c.reports, [date]: { data, timestamp: Date.now() } }
    })),

    // 设置配置
    setConfig: (data) => update(c => ({ ...c, config: data })),

    // 添加新活动到时间线缓存（增量更新）
    addActivity: (activity) => update(c => {
      const today = getLocalDateString();
      if (c.timeline[today]) {
        return {
          ...c,
          timeline: {
            ...c.timeline,
            [today]: {
              ...c.timeline[today],
              data: [activity, ...c.timeline[today].data],
              timestamp: Date.now(),
            }
          }
        };
      }
      return c;
    }),

    // 清除所有缓存
    clear: () => set({
      overview: { data: null, timestamp: 0, loading: false, date: null },
      timeline: {},
      reports: {},
      hourlySummaries: {},
      config: null,
    }),

    // 使缓存过期
    invalidate: (type, date = null) => update(c => {
      if (type === 'overview') {
        return { ...c, overview: { ...c.overview, timestamp: 0 } };
      }
      if (type === 'timeline' && date) {
        const timeline = { ...c.timeline };
        delete timeline[date];
        return { ...c, timeline };
      }
      if (type === 'report' && date) {
        const reports = { ...c.reports };
        delete reports[date];
        return { ...c, reports };
      }
      return c;
    }),
  };
}

export const cache = createCache();

// 导出便捷方法
export const getLocalDate = getLocalDateString;
