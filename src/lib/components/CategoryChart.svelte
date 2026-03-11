<script>
  import { onMount } from 'svelte';
  import Chart from 'chart.js/auto';

  export let data = [];
  export let total = 0;

  let canvas;
  let chart;

  // 分类名称映射
  const categoryNames = {
    development: '开发工具',
    browser: '浏览器',
    communication: '通讯协作',
    office: '办公软件',
    design: '设计工具',
    entertainment: '娱乐',
    other: '其他',
  };

  // 分类颜色
  const categoryColors = {
    development: '#3b82f6',
    browser: '#10b981',
    communication: '#f59e0b',
    office: '#8b5cf6',
    design: '#ec4899',
    entertainment: '#ef4444',
    other: '#6b7280',
  };

  onMount(() => {
    const ctx = canvas.getContext('2d');

    chart = new Chart(ctx, {
      type: 'doughnut',
      data: {
        labels: data.map(cat => categoryNames[cat.category] || cat.category),
        datasets: [{
          data: data.map(cat => cat.duration),
          backgroundColor: data.map(cat => categoryColors[cat.category] || '#6b7280'),
          borderWidth: 0,
          hoverOffset: 4,
        }],
      },
      options: {
        responsive: true,
        maintainAspectRatio: false,
        cutout: '60%',
        plugins: {
          legend: {
            position: 'right',
            labels: {
              usePointStyle: true,
              pointStyle: 'circle',
              padding: 16,
            },
          },
          tooltip: {
            callbacks: {
              label: (context) => {
                const cat = data[context.dataIndex];
                const percentage = total > 0 ? Math.round((cat.duration / total) * 100) : 0;
                const hours = Math.floor(cat.duration / 3600);
                const minutes = Math.floor((cat.duration % 3600) / 60);
                const timeStr = hours > 0 ? `${hours}h ${minutes}m` : `${minutes}m`;
                return `${timeStr} (${percentage}%)`;
              },
            },
          },
        },
      },
    });

    return () => {
      if (chart) {
        chart.destroy();
      }
    };
  });

  // 数据更新时重新渲染
  $: if (chart && data) {
    chart.data.labels = data.map(cat => categoryNames[cat.category] || cat.category);
    chart.data.datasets[0].data = data.map(cat => cat.duration);
    chart.data.datasets[0].backgroundColor = data.map(cat => categoryColors[cat.category] || '#6b7280');
    chart.update();
  }
</script>

<div class="h-64">
  <canvas bind:this={canvas}></canvas>
</div>
