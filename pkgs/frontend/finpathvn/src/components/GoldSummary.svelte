
<script>
  import { onMount } from 'svelte';

  export let initialData;
  let data = initialData;
  let isUpdating = false;

  async function refreshData() {
    isUpdating = true;
    try {
      const res = await fetch('YOUR_BACKEND_API_URL');
      if (res.ok) {
        data = await res.json();
      }
    } catch (error) {
      console.error("Lỗi cập nhật:", error);
    } finally {
      setTimeout(() => { isUpdating = false; }, 500);
    }
  }

  onMount(() => {
    const interval = setInterval(refreshData, 30000);
    return () => clearInterval(interval);
  });

  const getColor = (val) => {
    if (val?.includes('▲') || val?.includes('+')) return "text-green-700";
    if (val?.includes('▼') || val?.includes('-')) return "text-red-700";
    return "text-gray-500";
  };
</script>

<div class="grid grid-cols-1 md:grid-cols-2 gap-px bg-gray-200 border border-gray-200 shadow-sm overflow-hidden font-sans relative">
  {#if isUpdating}
    <div class="absolute top-0 left-0 w-full h-1 bg-green-500 animate-pulse z-10"></div>
  {/if}

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
        <p class="font-bold text-gray-800">{data.world.convertedVnd}</p>
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
