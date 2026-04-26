<script>
  import { onMount, onDestroy } from 'svelte';

  export let initialData;

  // Tách biệt data ngay từ đầu
  let dataVn = initialData.vn;
  let dataWorld = initialData.world;

  let socket;
  let sessionId = "qs_" + Math.random().toString(36).substring(2, 10);

  // --- Helper Functions ---
  const formatVN = (val) => {
    if (val === null || val === undefined || val === "" || isNaN(val)) return "---";
    return Number(val).toLocaleString('vi-VN');
  };

  const formatDiff = (val) => {
    if (!val || val === "") return "---";
    const hasArrowUp = String(val).includes('▲');
    const hasArrowDown = String(val).includes('▼');
    const numericValue = String(val).replace(/[▲▼\s]/g, '');

    if (numericValue === "" || isNaN(numericValue)) return val;
    const formattedNum = Number(numericValue).toLocaleString('vi-VN');

    if (hasArrowUp) return `▲ ${formattedNum}`;
    if (hasArrowDown) return `▼ ${formattedNum}`;
    return formattedNum;
  };

  // --- Live Update VN (Từ CustomEvent) ---
  function handleVNUpdate(event) {
    const allUpdates = event.detail;
    // index 0 là SJC Summary từ mảng allProductIds bên index.astro
    const myUpdate = allUpdates[0];

    if (myUpdate && !myUpdate.error) {
      dataVn.buy = formatVN(myUpdate.price?.buy || myUpdate.buy);
      dataVn.sell = formatVN(myUpdate.price?.sell || myUpdate.sell);
      dataVn.diffBuy = formatDiff(myUpdate.price?.diffBuy || myUpdate.diffBuy);
      dataVn.diffSell = formatDiff(myUpdate.price?.diffSell || myUpdate.diffSell);

      dataVn = dataVn; // Trigger re-render
    }
  }

  // --- Live Update Thế Giới (Websocket TradingView) ---
  const encodeTV = (obj) => {
    const msg = JSON.stringify(obj);
    return `~m~${msg.length}~m~${msg}`;
  };

  const decodeTV = (raw) => raw.split(/~m~\d+~m~/).filter(p => p.trim() && p.startsWith('{'));

  function connectWorld() {
    socket = new WebSocket("wss://widgetdata.tradingview.com/socket.io/websocket?from=embed-widget%2Fsingle-quote%2F&type=global");

    socket.onopen = () => {
      socket.send(encodeTV({ "m": "set_auth_token", "p": ["unauthorized_user_token"] }));
      socket.send(encodeTV({ "m": "quote_create_session", "p": [sessionId] }));
      socket.send(encodeTV({ "m": "quote_add_symbols", "p": [sessionId, "PEPPERSTONE:XAUUSD"] }));
    };

    socket.onmessage = (event) => {
      const raw = event.data;
      if (raw.includes('~h~')) { socket.send(raw); return; }

      const packets = decodeTV(raw);
      packets.forEach(async (packet) => {
        try {
          const json = JSON.parse(packet);
          if (json.m === 'qsd' && json.p[1].v) {
            const v = json.p[1].v;
            if (v.lp !== undefined) {
              dataWorld.price = v.lp.toLocaleString('en-US', { minimumFractionDigits: 2 });

              try {
                const res = await fetch('/api/investing/v2/exchange-rate');
                const rateJson = await res.json();
                const usdSell = parseFloat(rateJson.query.find(i => i.currencyCode === "USD")?.sell || "25000");

                const converted = (v.lp * 1.205 * usdSell) / 1000000;
                dataWorld.convertedVnd = converted.toLocaleString('vi-VN', { minimumFractionDigits: 2, maximumFractionDigits: 2 }) + " tr";
              } catch (apiErr) {
                console.error("API tỷ giá hắt hơi rồi:", apiErr);
              }
            }
            if (v.ch !== undefined) {
              dataWorld.diff = (v.ch > 0 ? '▲ ' : '▼ ') + Math.abs(v.ch).toFixed(2);
            }
            if (v.ch !== undefined && v.lp !== undefined) {
              const prevPrice = v.lp - v.ch;
              dataWorld.percent = ((v.ch / prevPrice) * 100).toFixed(2) + '%';
            }
            dataWorld = dataWorld; // Trigger re-render
          }
        } catch (e) {}
      });
    };

    socket.onclose = () => { if (socket) setTimeout(connectWorld, 5000); };
  }

  onMount(() => {
    connectWorld();
    window.addEventListener('price-update', handleVNUpdate);
  });

  onDestroy(() => {
    if (socket) {
      socket.onclose = null;
      socket.close();
    }
    if (typeof window !== 'undefined') {
      window.removeEventListener('price-update', handleVNUpdate);
    }
  });

  $: getColor = (val) => {
    const s = String(val);
    if (s.includes('▲') || s.includes('+')) return "text-green-700";
    if (s.includes('▼') || s.includes('-')) return "text-red-700";
    return "text-gray-500";
  };
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-px bg-gray-200 border border-gray-200 shadow-md rounded-xl overflow-hidden font-sans">
  <div class="bg-white p-4 md:p-5 flex flex-col justify-between">
    <div class="flex justify-between items-start mb-4">
      <div>
        <h3 class="text-[11px] md:text-sm font-bold text-gray-400 uppercase tracking-wider mb-1">Vàng miếng SJC</h3>
        <div class="flex items-center gap-2">
           <span class="bg-[#802237] text-white text-[10px] px-1.5 py-0.5 rounded font-bold uppercase">VN</span>
           <span class="text-[10px] md:text-xs text-gray-400 italic">triệu đồng/lượng</span>
        </div>
      </div>
    </div>

    <div class="flex flex-wrap justify-between items-end gap-y-4">
      <div class="min-w-fit">
        <p class="text-[10px] md:text-[12px] text-gray-400 font-bold uppercase mb-1">Bán ra</p>
        <div class="flex items-baseline gap-2">
          <span class="text-2xl md:text-3xl font-medium text-green-800 tracking-tighter leading-none">
            {dataVn.sell}
          </span>
          <span class="{getColor(dataVn.diffSell)} text-sm md:text-[12px] font-bold">
            {dataVn.diffSell}
          </span>
        </div>
      </div>

      <div class="min-w-fit text-right md:text-left md:border-l md:border-gray-100 md:pl-6">
        <p class="text-[10px] md:text-[12px] text-gray-400 font-bold uppercase mb-1">Mua vào</p>
        <div class="flex items-baseline justify-end md:justify-start gap-2">
          <span class="text-2xl md:text-3xl font-medium text-gray-800 tracking-tighter leading-none">
            {dataVn.buy}
          </span>
          <span class="{getColor(dataVn.diffBuy)} text-sm md:text-[12px] font-bold">
            {dataVn.diffBuy}
          </span>
        </div>
      </div>
    </div>
  </div>

  <div class="bg-white p-4 md:p-5 flex flex-col justify-between border-t border-gray-100 md:border-t-0">
    <div class="flex justify-between items-start mb-4">
      <div>
        <h3 class="text-[11px] md:text-sm font-bold text-gray-400 uppercase tracking-wider mb-1">Vàng Thế Giới</h3>
        <div class="flex items-center gap-2">
           <span class="bg-gray-800 text-white text-[10px] px-1.5 py-0.5 rounded font-bold uppercase">INTL</span>
           <span class="text-[10px] md:text-xs text-gray-400 italic">USD/Ounce</span>
        </div>
      </div>
      <div class="text-right">
        <span class="text-[11px] md:text-[13px] text-gray-500">Quy đổi VND:</span>
        <p class="font-medium text-gray-800 text-sm md:text-base">{dataWorld.convertedVnd || "0"}</p>
      </div>
    </div>

    <div>
      <p class="text-[10px] md:text-[12px] text-gray-400 font-bold uppercase mb-1">Giá hiện tại</p>
      <div class="flex items-baseline flex-wrap justify-between md:justify-start gap-2 md:gap-3">
        <span class="text-2xl md:text-3xl font-medium text-gray-800 tracking-tighter leading-none">
          {dataWorld.price}
        </span>

        <div class="flex items-baseline gap-1 md:gap-2">
           <span class="{getColor(dataWorld.diff)} text-sm md:text-[12px] font-bold">
             {dataWorld.diff}
           </span>
           <span class="{getColor(dataWorld.percent)} text-sm md:text-[12px] font-bold">
             ({dataWorld.percent})
           </span>
        </div>
      </div>
    </div>
  </div>
</div>
