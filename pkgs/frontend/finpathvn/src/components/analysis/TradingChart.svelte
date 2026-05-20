<script>
  import { onMount, onDestroy } from 'svelte';
  import { init } from 'klinecharts';

  let chart = null;
  let pollingInterval = null;
  const stopMap = new Map();

  function normalizeToKLineData(data) {
    return {
      timestamp: data.t * 1000,   // nếu API trả seconds → đổi sang ms
      open: Number(data.o),
      high: Number(data.h),
      low: Number(data.l),
      close: Number(data.c),
      volume: Number(data.v),
    };
  }

  function makeKey(symbol, period) {
    return `${symbol.ticker}_${period.span}${period.type}`;
  }

  onMount(() => {
    chart = init('chart', {
      grid: { show: true },
    });

    chart.setSymbol({ ticker: 'BTC/USDT' }); // ← thay bằng symbol của bạn
    chart.setPeriod({ span: 1, type: 'day' });

    chart.setDataLoader({
      async getBars({ type, timestamp, symbol, period, callback }) {
        try {
          // ⚠️ Thay bằng API thực của bạn
          const response = await api.getKlineList({
            symbol: symbol.ticker,
            period: `${period.span}${period.type}`,
            endTime: timestamp ?? Date.now(),
            limit: 500,
            direction: type,
          });

          const bars = response.list
            .map(normalizeToKLineData)
            .sort((a, b) => a.timestamp - b.timestamp);

          callback(bars, {
            forward: response.hasMoreBefore,
            backward: response.hasMoreAfter,
          });
        } catch (err) {
          console.error('getBars error:', err);
          callback([], { forward: false, backward: false });
        }
      },

    });
  });

  onDestroy(() => {
    // Dọn hết polling còn sót
    stopMap.forEach((stop) => stop());
    stopMap.clear();

    if (chart) {
      chart.dispose();
      chart = null;
    }
  });
</script>

<div id="chart" style="width: 600px; height: 600px; border: 1px solid #ddd;" />
