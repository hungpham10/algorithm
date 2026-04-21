<script>
  import { onMount, onDestroy } from 'svelte';

  export let initialData;
  let data = initialData;
  let socket;
  let sessionId = "qs_" + Math.random().toString(36).substring(2, 10);

  const encodeTV = (obj) => {
    const msg = JSON.stringify(obj);
    return `~m~${msg.length}~m~${msg}`;
  };

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
      if (raw.includes('~h~')) {
        socket.send(raw);
        return;
      }

      const packets = decodeTV(raw);
      packets.forEach(packet => {
        try {
          const json = JSON.parse(packet);
          if (json.m === 'qsd' && json.p.v) {
            const v = json.p.v;
            if (v.lp !== undefined) {
              data.world.price = v.lp.toLocaleString('en-US', { minimumFractionDigits: 2 });
            }
            if (v.ch !== undefined) {
              data.world.diff = (v.ch > 0 ? '▲ ' : '▼ ') + Math.abs(v.ch).toFixed(2);
            }
            if (v.ch !== undefined && v.lp !== undefined) {
              const prevPrice = v.lp - v.ch;
              data.world.percent = ((v.ch / prevPrice) * 100).toFixed(2) + '%';
            }
          }
        } catch (e) {}
      });
    };

    socket.onclose = () => setTimeout(connect, 5000);
    socket.onerror = () => socket.close();
  }

  onMount(() => { connect(); });
  onDestroy(() => { if (socket) socket.close(); });

  const getColor = (val) => {
    if (val?.includes('▲') || val?.includes('+')) return "text-green-700";
    if (val?.includes('▼') || val?.includes('-')) return "text-red-700";
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
      <div class="text-right">
        <span class="text-[11px] md:text-[13px] text-gray-500">Chênh lệch TG:</span>
        <p class="font-bold text-gray-800 text-sm md:text-base">+{data.vn.gap}</p>
      </div>
    </div>

    <div class="flex flex-wrap justify-between items-end gap-y-4">
      <div class="min-w-fit">
        <p class="text-[10px] md:text-[12px] text-gray-400 font-bold uppercase mb-1">Bán ra</p>
        <div class="flex items-baseline gap-2">
          <span class="text-3xl md:text-4xl font-bold text-green-700 tracking-tighter leading-none">
            {data.vn.sell}
          </span>
          <span class="{getColor(data.vn.diffSell)} text-sm md:text-[15px] font-bold">
            {data.vn.diffSell}
          </span>
        </div>
      </div>

      <div class="min-w-fit text-right md:text-left md:border-l md:border-gray-100 md:pl-6">
        <p class="text-[10px] md:text-[12px] text-gray-400 font-bold uppercase mb-1">Mua vào</p>
        <div class="flex items-baseline justify-end md:justify-start gap-2">
          <span class="text-3xl md:text-4xl font-bold text-gray-900 tracking-tighter leading-none">
            {data.vn.buy}
          </span>
          <span class="{getColor(data.vn.diffBuy)} text-sm md:text-[15px] font-bold">
            {data.vn.diffBuy}
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
        <p class="font-bold text-gray-800 text-sm md:text-base">{data.world.convertedVnd || "0"}</p>
      </div>
    </div>

    <div>
      <p class="text-[10px] md:text-[12px] text-gray-400 font-bold uppercase mb-1">Giá hiện tại</p>
      <div class="flex items-baseline flex-wrap justify-between md:justify-start gap-2 md:gap-3">
        <span class="text-3xl md:text-4xl font-bold text-gray-900 tracking-tighter leading-none">
          {data.world.price}
        </span>

        <div class="flex items-baseline gap-1 md:gap-2">
           <span class="{getColor(data.world.diff)} text-sm md:text-[18px] font-bold">
             {data.world.diff}
           </span>
           <span class="{getColor(data.world.percent)} text-sm md:text-[18px] font-bold">
             ({data.world.percent})
           </span>
        </div>
      </div>
    </div>
  </div>
</div>
