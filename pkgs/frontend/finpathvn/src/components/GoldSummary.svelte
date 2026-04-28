<script>
  import { onMount, onDestroy } from 'svelte';

  export let initialData;

  // Tách biệt data ngay từ đầu
  let dataVn = initialData.vn;
  let dataWorld = initialData.world;

  let socket;
  let sessionId = "qs_" + Math.random().toString(36).substring(2, 10);

  // --- Helper Functions ---
  function formatVN(val) {
    if (val === null || val === undefined || val === "" || isNaN(val)) return "---";
    return Number(val).toLocaleString('vi-VN');
  }

  function formatDiff(val) {
    if (val === null || val === undefined || val === "") return "---";

    // Nếu backend trả về số kèm mũi tên, ta xử lý để format phần số
    const hasNotArrow = String(val).includes('●');
    const hasArrowUp = String(val).includes('▲');
    const hasArrowDown = String(val).includes('▼');
    const numericValue = String(val).replace(/[▲▼●\s]/g, ''); // Loại bỏ ký tự lạ để lấy số

    if (numericValue === "" || isNaN(numericValue)) return val; // Nếu không phải số thì trả về nguyên bản
    const formattedNum = Number(numericValue).toLocaleString('vi-VN');

    if (!hasArrowUp && !hasArrowDown) {
      if (numericValue > 0) {
    	return `▲ ${formattedNum}`;
      } else if (numericValue < 0) {
    	return `▼ ${formattedNum}`;
      } else {
    	return `${formattedNum}`;
      }
    }

    if (hasNotArrow) return `${formattedNum}`;
    if (hasArrowUp) return `▲ ${formattedNum}`;
    if (hasArrowDown) return `▼ ${formattedNum}`;
    return formattedNum;
  }

  // --- Live Update VN (Từ CustomEvent) ---
  function handleVNUpdate(event) {
    const allUpdates = event.detail;
    // index 0 là SJC Summary từ mảng allProductIds bên index.astro
    const myUpdate = allUpdates[0];

    if (myUpdate && !myUpdate.error) {
      dataVn.buy = formatVN(myUpdate.price?.buy);
      dataVn.sell = formatVN(myUpdate.price?.sell);
      dataVn.diffBuy = formatDiff(myUpdate.price?.diff[0]);
      dataVn.diffSell = formatDiff(myUpdate.price?.diff[1]);

      console.log(dataVn)
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

<div class="grid grid-cols-1 md:grid-cols-2 bg-white border border-gray-200 rounded-lg overflow-hidden font-sans mt-2">

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
