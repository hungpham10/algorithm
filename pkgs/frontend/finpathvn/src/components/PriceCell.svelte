<script>
  import { onMount } from 'svelte';

  export let productId; // Nhận ID từ TableRow.astro
  export let type = "buy";
  export let initialPrice = "";
  export let initialDiff = "";

  let price = initialPrice;
  let diff = initialDiff;

  // Logic màu sắc giữ nguyên
  $: isUp = diff?.includes('▲');
  $: isDown = diff?.includes('▼');
  $: colorClass = isUp ? "text-green-700" : isDown ? "text-red-700" : "text-gray-500";
  $: priceColor = (type === "sell") ? colorClass : "text-gray-900";

  // Hàm xử lý khi nhận được "bản tin" từ index.astro
  function handleLiveUpdate(event) {
    const allUpdates = event.detail; // Mảng các OhclResponse

    // Tìm đúng dữ liệu của productId này
    const myUpdate = allUpdates.find(item => item.product_id === productId);

    if (myUpdate) {
      if (myUpdate.error) {
        // Nếu backend báo lỗi cho ID này
        price = "---";
        diff = "Lỗi";
      } else {
        // Cập nhật giá trị mới
        price = type === "buy" ? myUpdate.buy : myUpdate.sell;
        // Đảm bảo Backend trả về field diffBuy/diffSell trong OhclResponse
        diff = type === "buy" ? myUpdate.diffBuy : myUpdate.diffSell;
      }
    }
  }

  onMount(() => {
    // Đăng ký lắng nghe sự kiện chung
    window.addEventListener('price-update', handleLiveUpdate);

    return () => {
      // Hủy lắng nghe khi component bị hủy (tránh rò rỉ bộ nhớ)
      window.removeEventListener('price-update', handleLiveUpdate);
    };
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
