
<script>
  import { onMount } from 'svelte';

  export let initialPrice = "";
  export let initialDiff = "";
  export let type = "buy";
  export let goldType = "";

  let price = initialPrice;
  let diff = initialDiff;

  // Logic màu sắc giữ nguyên như bạn đã thiết lập
  $: isUp = diff?.includes('▲');
  $: isDown = diff?.includes('▼');
  $: colorClass = isUp ? "text-green-700" : isDown ? "text-red-700" : "text-gray-500";
  $: priceColor = (type === "sell") ? colorClass : "text-gray-900";

  async function updateData() {
    try {
      const res = await fetch(`/api/gold-single?type=${encodeURIComponent(goldType)}`);
      if (res.ok) {
        const newData = await res.json();
        price = type === "buy" ? newData.buy : newData.sell;
        diff = type === "buy" ? newData.diffBuy : newData.diffSell;
      }
    } catch (e) {
      console.error("Lỗi cập nhật");
    }
  }

  onMount(() => {
    const interval = setInterval(updateData, 15000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex flex-col items-end leading-tight">
  <div class="font-medium text-[18px] md:text-[24px] tracking-tighter {priceColor}">
    {price}
  </div>

  <div class="text-[11px] md:text-[14px] font-normal {colorClass} -mt-0.5 md:mt-0">
    {diff}
  </div>
</div>
