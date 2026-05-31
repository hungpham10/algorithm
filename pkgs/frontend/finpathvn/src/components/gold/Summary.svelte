
<script>
  import { onMount, onDestroy } from 'svelte';
  import { PRICE_EVENT_NAME } from '$lib/schema.js';
  import { formatVN, formatDiff } from '$lib/utils.js';

  export let initialData;

  let dataVn = initialData.vn;
  let dataWorld = initialData.world;

  let socket;
  let usdSellRate = 25000;
  let lastRateFetch = 0;
  const RATE_CACHE_TIME = 5 * 60 * 1000; // 5 phút

  // --- Live Update VN ---
  function handleVNUpdate(event) {
    const allUpdates = event.detail;
    const myUpdate = allUpdates[0];
    if (myUpdate && !myUpdate.error) {
      dataVn.buy = formatVN(myUpdate.buy);
      dataVn.sell = formatVN(myUpdate.sell);
      dataVn.diffBuy = formatDiff(myUpdate.diffBuy);
      dataVn.diffSell = formatDiff(myUpdate.diffSell);
      dataVn = dataVn;
    }
  }

  // --- Exchange Rate Cache ---
  async function updateExchangeRate() {
    const now = Date.now();
    if (now - lastRateFetch < RATE_CACHE_TIME) return usdSellRate;

    try {
      const response = await fetch('/api/investing/v2/exchang-rate');
      if (response.ok) {
        const rateJson = await response.json();
        const rate = rateJson.query.find(i => i.currencyCode === "USD")?.sell;
        if (rate) {
          usdSellRate = parseFloat(rate);
          lastRateFetch = now;
        }
      }
    } catch (e) {
      console.error("Lỗi lấy tỷ giá:", e);
    }
    return usdSellRate;
  }

  // --- Live Update World ---
  function connectWorld() {
    const protocol = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
    const wsUrl = `${protocol}${window.location.host}/ws/investing/v3`;

    socket = new WebSocket(wsUrl);

    socket.onopen = () => {
      socket.send(JSON.stringify({
        action: "subscribe",
        symbols: [{ broker: "simplefx", symbol: "XAUUSD" }]
      }));
    };

    socket.onmessage = async (event) => {
      try {
        const data = JSON.parse(event.data);
        if (data.event === 'tick' && data.symbol === 'XAUUSD' && data.price) {
          const currentPrice = data.price;
          const previousPrice = parseFloat(dataWorld.price?.replace(/,/g, '')) || currentPrice;

          dataWorld.price = currentPrice.toLocaleString('en-US', { minimumFractionDigits: 2 });

          const usdRate = await updateExchangeRate();
          const converted = (currentPrice * 1.205 * usdRate) / 1000000;
          dataWorld.convertedVnd = converted.toLocaleString('vi-VN', {
            minimumFractionDigits: 2,
            maximumFractionDigits: 2
          }) + " tr";

          const diffValue = currentPrice - previousPrice;
          if (diffValue !== 0) {
            dataWorld.diff = (diffValue > 0 ? '▲ ' : '▼ ') + Math.abs(diffValue).toFixed(2);
            dataWorld.percent = ((diffValue / previousPrice) * 100).toFixed(2) + '%';
          }

          dataWorld = dataWorld;
        }
      } catch (e) {
        console.error("Lỗi parse tick:", e);
      }
    };

    socket.onclose = () => {
      setTimeout(connectWorld, 5000);
    };
  }

  onMount(() => {
    connectWorld();
    window.addEventListener(PRICE_EVENT_NAME, handleVNUpdate);
  });

  onDestroy(() => {
    if (socket) socket.close();
    window.removeEventListener(PRICE_EVENT_NAME, handleVNUpdate);
  });

  $: getColor = (val) => {
    const s = String(val || '');
    if (s.includes('▲') || s.includes('+')) return "text-emerald-600";
    if (s.includes('▼') || s.includes('-')) return "text-red-600";
    return "text-gray-500";
  };
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-4">

  <!-- Việt Nam Card -->
  <div class="surface-card rounded-2xl p-5 relative overflow-hidden bg-white border border-gray-100">
    <div class="flex items-center justify-between">
      <span class="text-[10px] uppercase tracking-[0.18em] text-blue-600 font-semibold flex items-center gap-1.5">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
          <path d="M13.744 17.736a6 6 0 1 1-7.48-7.48"></path>
          <path d="M15 6h1v4"></path>
          <path d="m6.134 14.768.866-.5 2 3.464"></path>
          <circle cx="16" cy="8" r="6"></circle>
        </svg>
        Việt Nam
      </span>
      <span class="text-xs font-semibold {getColor(dataVn.diffSell)}">+0.32%</span>
    </div>

    <div class="mt-3">
      <div class="font-serif text-xl leading-tight">Vàng miếng SJC</div>
      <div class="text-[11px] uppercase tracking-wider text-gray-500 mt-1">Triệu đồng/lượng</div>
    </div>

    <div class="mt-5 grid grid-cols-2 gap-3">
      <div>
        <div class="text-[10px] uppercase tracking-wider text-gray-500">Mua vào</div>
        <div class="font-mono text-2xl mt-1 tabular-nums">{dataVn.buy}</div>
      </div>
      <div>
        <div class="text-[10px] uppercase tracking-wider text-gray-500">Bán ra</div>
        <div class="font-mono text-2xl mt-1 tabular-nums text-blue-600">{dataVn.sell}</div>
      </div>
    </div>

    <!-- Decorative blur circle -->
    <div class="absolute -right-10 -bottom-10 w-32 h-32 rounded-full bg-blue-500/10 blur-2xl"></div>
  </div>

  <!-- World Card -->
  <div class="surface-card rounded-2xl p-5 relative overflow-hidden bg-white border border-gray-100">
    <div class="flex items-center justify-between">
      <span class="text-[10px] uppercase tracking-[0.18em] text-amber-600 font-semibold flex items-center gap-1.5">
        <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="w-4 h-4">
          <path d="M21.54 15H17a2 2 0 0 0-2 2v4.54"></path>
          <path d="M7 3.34V5a3 3 0 0 0 3 3a2 2 0 0 1 2 2c0 1.1.9 2 2 2a2 2 0 0 0 2-2c0-1.1.9-2 2-2h3.17"></path>
          <path d="M11 21.95V18a2 2 0 0 0-2-2a2 2 0 0 1-2-2v-1a2 2 0 0 0-2-2H2.05"></path>
          <circle cx="12" cy="12" r="10"></circle>
        </svg>
        World
      </span>
      <span class="text-xs font-semibold {getColor(dataWorld.diff)}">+0.18%</span>
    </div>

    <div class="mt-3">
      <div class="font-serif text-xl leading-tight">Vàng thế giới</div>
      <div class="text-[11px] uppercase tracking-wider text-gray-500 mt-1">USD/Ounce</div>
    </div>

    <div class="mt-5 grid grid-cols-2 gap-3">
      <div>
        <div class="text-[10px] uppercase tracking-wider text-gray-500">Giá</div>
        <div class="font-mono text-2xl mt-1 tabular-nums">{dataWorld.price}</div>
      </div>
      <div>
        <div class="text-[10px] uppercase tracking-wider text-gray-500">Quy đổi VND</div>
        <div class="font-mono text-2xl mt-1 tabular-nums text-blue-600">{dataWorld.convertedVnd || '—'}</div>
      </div>
    </div>

    <!-- Decorative blur circle -->
    <div class="absolute -right-10 -bottom-10 w-32 h-32 rounded-full bg-amber-500/10 blur-2xl"></div>
  </div>
</div>
