
<script>
  import { onMount, onDestroy } from 'svelte';

  export let initialData;
  let data = initialData;
  let socket;
  let sessionId = "qs_" + Math.random().toString(36).substring(2, 10);

  // Gói tin nhắn: ~m~len~m~json
  const encodeTV = (obj) => {
    const msg = JSON.stringify(obj);
    return `~m~${msg.length}~m~${msg}`;
  };

  // Giải mã: Tách các gói tin dựa trên marker ~m~
  const decodeTV = (raw) => {
    return raw.split(/~m~\d+~m~/).filter(p => p.trim() && p.startsWith('{'));
  };

  function connect() {
    socket = new WebSocket("wss://widgetdata.tradingview.com/socket.io/websocket?from=embed-widget%2Fsingle-quote%2F&type=global");

    socket.onopen = () => {
      socket.send(encodeTV({ "m": "set_auth_token", "p": ["unauthorized_user_token"] }));
      socket.send(encodeTV({ "m": "quote_create_session", "p": [sessionId] }));
      socket.send(encodeTV({
        "m": "quote_add_symbols",
        "p": [sessionId, "PEPPERSTONE:XAUUSD"]
      }));
    };

    socket.onmessage = (event) => {
      const raw = event.data;

      // Heartbeat: Server gửi gì, trả lại nấy (ví dụ ~m~4~m~~h~2)
      if (raw.includes('~h~')) {
        socket.send(raw);
        return;
      }

      const packets = decodeTV(raw);
      packets.forEach(packet => {
        try {
          const json = JSON.parse(packet);

          // m: "qsd" là gói tin dữ liệu giá
          if (json.m === 'qsd' && json.p[1].v) {
            const v = json.p[1].v;

            if (v.lp !== undefined) {
              data.world.price = v.lp.toLocaleString('en-US', { minimumFractionDigits: 2 });
            }
            if (v.ch !== undefined) {
              data.world.diff = (v.ch > 0 ? '▲ ' : '▼ ') + Math.abs(v.ch).toFixed(2);
            }
            // Tự tính % nếu gói tin không có chp
            if (v.ch !== undefined && v.lp !== undefined) {
              const prevPrice = v.lp - v.ch;
              data.world.percent = ((v.ch / prevPrice) * 100).toFixed(2) + '%';
            }
          }
        } catch (e) {
          // Bỏ qua các gói tin không phải JSON hợp lệ
        }
      });
    };

    socket.onclose = () => setTimeout(connect, 5000);
    socket.onerror = () => socket.close();
  }

  onMount(() => {
    connect();
  });

  onDestroy(() => {
    if (socket) socket.close();
  });

  const getColor = (val) => {
    if (val?.includes('▲') || val?.includes('+')) return "text-green-700";
    if (val?.includes('▼') || val?.includes('-')) return "text-red-700";
    return "text-gray-500";
  };
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-px bg-gray-200 border border-gray-200 shadow-sm overflow-hidden font-sans relative">
  <div class="bg-white p-5 flex flex-col justify-between">
    <div class="flex justify-between items-start mb-4">
      <div>
        <h3 class="text-sm font-bold text-gray-400 uppercase tracking-wider mb-1">Vàng miếng SJC</h3>
        <div class="flex items-center gap-2">
           <span class="bg-[#802237] text-white text-[10px] px-1.5 py-0.5 rounded font-bold">VN</span>
           <span class="text-xs text-gray-400 italic">triệu đồng/lượng</span>
        </div>
      </div>
      <div class="text-right">
        <span class="text-[13px] text-gray-500">Chênh lệch TG:</span>
        <p class="font-bold text-gray-800">+{data.vn.gap}</p>
      </div>
    </div>

    <div class="grid grid-cols-2 gap-4">
      <div>
        <p class="text-[12px] text-gray-400 font-bold uppercase">Bán ra</p>
        <div class="flex items-baseline gap-1.5">
          <span class="text-4xl font-medium text-green-700 tracking-tight">{data.vn.sell}</span>
          <span class="{getColor(data.vn.diffSell)} text-[13px] font-bold">{data.vn.diffSell}</span>
        </div>
      </div>
      <div class="border-l border-gray-100 pl-4">
        <p class="text-[12px] text-gray-400 font-bold uppercase">Mua vào</p>
        <div class="flex items-baseline gap-1.5">
          <span class="text-4xl font-medium text-gray-900 tracking-tight">{data.vn.buy}</span>
          <span class="{getColor(data.vn.diffBuy)} text-[13px] font-bold">{data.vn.diffBuy}</span>
        </div>
      </div>
    </div>
  </div>

  <div class="bg-white p-5 flex flex-col justify-between">
    <div class="flex justify-between items-start mb-4">
      <div>
        <h3 class="text-sm font-bold text-gray-400 uppercase tracking-wider mb-1">Vàng Thế Giới</h3>
        <div class="flex items-center gap-2">
           <span class="bg-gray-800 text-white text-[10px] px-1.5 py-0.5 rounded font-bold">INTL</span>
           <span class="text-xs text-gray-400 italic">USD/Ounce</span>
        </div>
      </div>
      <div class="text-right">
        <span class="text-[13px] text-gray-500">Quy đổi VND:</span>
        <p class="font-bold text-gray-800">{data.world.convertedVnd || "0"}</p>
      </div>
    </div>

    <div>
      <p class="text-[12px] text-gray-400 font-bold uppercase">Giá hiện tại</p>
      <div class="flex items-baseline gap-2">
        <span class="text-4xl font-medium text-gray-900 tracking-tight">{data.world.price}</span>
        <div class="flex flex-col">
           <span class="{getColor(data.world.diff)} text-[13px] font-bold leading-none">{data.world.diff}</span>
           <span class="{getColor(data.world.percent)} text-[11px] font-medium">{data.world.percent}</span>
        </div>
      </div>
    </div>
  </div>
</div>
