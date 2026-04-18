<script>
  import { onMount } from 'svelte';

  export let initialPrice = "";
  export let initialDiff = "";
  export let type = "buy";
  export let goldType = "";
  export let liveEndpoint = "";

  let price = initialPrice;
  let diff = initialDiff;

  // Logic màu sắc
  $: isUp = diff?.includes('▲');
  $: isDown = diff?.includes('▼');
  $: colorClass = isUp ? "text-green-700" : isDown ? "text-red-700" : "text-gray-500";
  $: priceColor = (type === "sell") ? colorClass : "text-gray-900";

  async function updateData() {
    if (!liveEndpoint) return;
    try {
      const res = await fetch(liveEndpoint);

      if (res.ok) {
        const newData = await res.json();
        // Cập nhật dữ liệu mới nếu thành công
        price = type === "buy" ? newData.buy : newData.sell;
        diff = type === "buy" ? newData.diffBuy : newData.diffSell;
      } else {
        // Nếu HTTP status không ok (ví dụ 404, 500)
        throw new Error("Server error");
      }
    } catch (e) {
      console.error("Lỗi cập nhật:", e);
      // Khi lỗi (mất mạng hoặc server sập), trả về "---"
      price = "---";
      diff = "---";
    }
  }

  onMount(() => {
    const interval = setInterval(updateData, 15000);
    return () => clearInterval(interval);
  });
</script>

<div class="flex flex-col items-end leading-tight">
  <div class="font-medium text-[18px] md:text-[24px] tracking-tighter {priceColor}">
    {price || "---"}
  </div>

  <div class="text-[11px] md:text-[14px] font-normal {colorClass} -mt-0.5 md:mt-0">
    {diff || "---"}
  </div>
</div>
