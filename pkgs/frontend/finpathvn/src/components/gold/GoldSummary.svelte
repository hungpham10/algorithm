
<script>
  import { onMount, onDestroy } from 'svelte';
  import { PRICE_EVENT_NAME } from '$lib/schema.js';
  import { formatVN, formatDiff } from '$lib/utils.js';

  export let initialData;

  // Tách biệt data ngay từ đầu
  let dataVn = initialData.vn;
  let dataWorld = initialData.world;

  let socket;

  // Cache tỷ giá để tránh spam API
  let usdSellRate = 25000;
  let lastRateFetch = 0;
  const RATE_CACHE_TIME = 5 * 60 * 1000; // 5 phút cập nhật 1 lần

  // --- Live Update VN (Từ CustomEvent) ---
  function handleVNUpdate(event) {
    const allUpdates = event.detail;
    const myUpdate = allUpdates[0];

    if (myUpdate && !myUpdate.error) {
      dataVn.buy = formatVN(myUpdate.buy);
      dataVn.sell = formatVN(myUpdate.sell);
      dataVn.diffBuy = formatDiff(myUpdate.diffBuy);
      dataVn.diffSell = formatDiff(myUpdate.diffSell);
      dataVn = dataVn; // Trigger re-render
    }
  }

  // --- Hàm bổ trợ lấy tỷ giá (Có kiểm tra Cache) ---
  async function updateExchangeRate() {
    const now = Date.now();
    if (now - lastRateFetch < RATE_CACHE_TIME) return usdSellRate;

    try {
      const response = await fetch('/api/investing/v2/exchange-rate');
      if (response.ok) {
        const rateJson = await response.json();
        const rate = rateJson.query.find(i => i.currencyCode === "USD")?.sell;
        if (rate) {
          usdSellRate = parseFloat(rate);
          lastRateFetch = now;
        }
      }
    } catch (apiErr) {
      console.error("API tỷ giá gặp lỗi:", apiErr);
    }
    return usdSellRate;
  }

  // --- Live Update Thế Giới (Websocket Mới) ---
  function connectWorld() {
    // Tự động nhận diện giao thức ws/wss và host hiện tại của ứng dụng
    const protocol = window.location.protocol === 'https:' ? 'wss://' : 'ws://';
    const wsUrl = `${protocol}${window.location.host}/ws/investing/v3`;

    socket = new WebSocket(wsUrl);

    socket.onopen = () => {
      // Gửi sub đúng format bạn yêu cầu
      socket.send(JSON.stringify({
        "action": "subscribe",
        "symbols": [
          { "broker": "simplefx", "symbol": "XAUUSD" }
        ]
      }));
    };

    socket.onmessage = async (event) => {
      try {
        const data = JSON.parse(event.data);

        // Khớp cấu trúc event: "tick" từ broker và symbol của bạn
        if (data.event === 'tick' && data.symbol === 'XAUUSD' && data.price) {
          const currentPrice = data.price;

          // 1. Lưu giá cũ để tính diff trước khi ghi đè giá mới
          const previousPrice = parseFloat(dataWorld.price?.replace(/,/g, '')) || currentPrice;

          // Định dạng hiển thị giá hiện tại (Ví dụ: 4,505.77)
          dataWorld.price = currentPrice.toLocaleString('en-US', { minimumFractionDigits: 2 });

          // 2. Lấy tỷ giá từ cache/API và tính giá quy đổi
          const usdRate = await updateExchangeRate();
          const converted = (currentPrice * 1.205 * usdRate) / 1000000;
          dataWorld.convertedVnd = converted.toLocaleString('vi-VN', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + " tr";

          // 3. Tính toán độ lệch tăng giảm (Biến động thực tế giữa các tick)
          const diffValue = currentPrice - previousPrice;
          if (diffValue !== 0) {
            dataWorld.diff = (diffValue > 0 ? '▲ ' : '▼ ') + Math.abs(diffValue).toFixed(2);
            dataWorld.percent = ((diffValue / previousPrice) * 100).toFixed(2) + '%';
          }

          dataWorld = dataWorld; // Trigger re-render Svelte
        }
      } catch (e) {
        console.error("Lỗi parse dữ liệu mạng:", e);
      }
    };

    socket.onclose = () => {
      if (socket) setTimeout(connectWorld, 5000);
    };
  }

  onMount(() => {
    connectWorld();
    window.addEventListener(PRICE_EVENT_NAME, handleVNUpdate);
  });

  onDestroy(() => {
    if (socket) {
      socket.onclose = null;
      socket.close();
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener(PRICE_EVENT_NAME, handleVNUpdate);
    }
  });

  $: getColor = (val) => {
    const s = String(val);
    if (s.includes('▲') || s.includes('+')) return "text-green-700";
    if (s.includes('▼') || s.includes('-')) return "text-red-700";
    return "text-gray-500";
  };
</script>

<div class="grid grid-cols-1 md:grid-cols-2 bg-white border border-gray-200 rounded-lg overflow-hidden font-sans mt-2">
  <!-- Giữ nguyên phần HTML giao diện của bạn bên dưới -->
  <div class="p-4 md:p-6 flex flex-col justify-between border-b md:border-b-0 md:border-r border-gray-200">
    <div class="flex justify-between items-start mb-6">
      <div>
        <div class="flex items-center gap-2 mb-1">
           <span class="bg-gray-100 text-gray-600 text-[9px] px-1.5 py-0.5 rounded font-bold uppercase tracking-widest">Việt Nam</span>
           <h3 class="text-[11px] font-bold text-gray-400 uppercase tracking-wider">Vàng miếng SJC</h3>
        </div>
        <span class="text-[10px] text-gray-300 italic uppercase">triệu đồng/lượng</span>
      </div>
    </div>

    <div class="grid grid-cols-2 gap-4">
      <div class="min-w-fit">
        <p class="text-[10px] text-gray-400 font-bold uppercase mb-1.5">Bán ra</p>
        <div class="flex items-baseline gap-2">
          <span class="text-2xl md:text-3xl font-semibold text-gray-900 tracking-tighter leading-none">
            {dataVn.sell}
          </span>
          <span class="{getColor(dataVn.diffSell)} text-[11px] font-bold">
            {dataVn.diffSell}
          </span>
        </div>
      </div>

      <div class="min-w-fit border-l border-gray-100 pl-4">
        <p class="text-[10px] text-gray-400 font-bold uppercase mb-1.5">Mua vào</p>
        <div class="flex items-baseline gap-2">
          <span class="text-2xl md:text-3xl font-semibold text-gray-800 tracking-tighter leading-none">
            {dataVn.buy}
          </span>
          <span class="{getColor(dataVn.diffBuy)} text-[11px] font-bold">
            {dataVn.diffBuy}
          </span>
        </div>
      </div>
    </div>
  </div>

  <div class="p-4 md:p-6 flex flex-col justify-between bg-[#f9fafb]">
    <div class="flex justify-between items-start mb-6">
      <div>
        <div class="flex items-center gap-2 mb-1">
           <span class="bg-gray-800 text-white text-[9px] px-1.5 py-0.5 rounded font-bold uppercase tracking-widest">World</span>
           <h3 class="text-[11px] font-bold text-gray-400 uppercase tracking-wider">Vàng Thế Giới</h3>
        </div>
        <span class="text-[10px] text-gray-300 italic uppercase">USD/Ounce</span>
      </div>
      <div class="text-right">
        <span class="text-[10px] font-bold text-gray-400 uppercase tracking-tight">Quy đổi VND</span>
        <p class="font-bold text-gray-700 text-sm md:text-base leading-tight">{dataWorld.convertedVnd || "0"}</p>
      </div>
    </div>

    <div>
      <p class="text-[10px] text-gray-400 font-bold uppercase mb-1.5">Giá hiện tại</p>
      <div class="flex items-baseline gap-3">
        <span class="text-2xl md:text-3xl font-semibold text-gray-900 tracking-tighter leading-none">
          {dataWorld.price}
        </span>

        <div class="flex items-baseline gap-1.5">
           <span class="{getColor(dataWorld.diff)} text-[11px] font-bold">
             {dataWorld.diff}
           </span>
           <span class="{getColor(dataWorld.percent)} text-[11px] font-bold">
             ({dataWorld.percent})
           </span>
        </div>
      </div>
    </div>
  </div>
</div>
