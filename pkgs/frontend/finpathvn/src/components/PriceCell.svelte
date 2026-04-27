<script>
  import { onMount } from 'svelte';

  export let index;
  export let type = "buy";
  export let initialPrice = "";
  export let initialDiff = "";

  // Hàm định dạng số kiểu Việt Nam
  function formatVN(val) {
    if (val === null || val === undefined || val === "" || isNaN(val)) return "---";
    return Number(val).toLocaleString('vi-VN');
  }

  // Hàm xử lý logic hiển thị Diff (giữ lại ký tự mũi tên nếu có)
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
    	return `● ${formattedNum}`;
      }
    }

    if (hasNotArrow) return `● ${formattedNum}`;
    if (hasArrowUp) return `▲ ${formattedNum}`;
    if (hasArrowDown) return `▼ ${formattedNum}`;
    return formattedNum;
  }

  let price = formatVN(initialPrice);
  let diff = initialDiff;

  // Logic màu sắc
  $: isUp = String(diff).includes('▲');
  $: isDown = String(diff).includes('▼');
  $: isNoChange = String(diff).includes('●');
  $: colorClass = isUp ? "text-green-700" : isDown ? "text-red-700" : "text-gray-900";
  $: priceColor = (type === "sell") ? colorClass : "text-gray-900";

  function handleLiveUpdate(event) {
    const allUpdates = event.detail;

    // Sử dụng index để lấy data trực tiếp
    const myUpdate = allUpdates[index];

    if (myUpdate) {
      if (myUpdate.error) {
        price = "---";
      } else {
        price = formatVN(type === "buy" ? myUpdate.price.buy : myUpdate.price.sell);
	diff = formatDiff(type === "buy"? myUpdate.price.diff[0]: myUpdate.price.diff[1]);
      }
    }
  }
  onMount(() => {
    window.addEventListener('price-update', handleLiveUpdate);
    return () => {
      window.removeEventListener('price-update', handleLiveUpdate);
    };
  });
</script>

<div class="flex flex-col items-center justify-center leading-tight mx-auto">
  <div class="font-medium text-[18px] md:text-[18px] tracking-tight {priceColor}">
    {price}
  </div>
</div>
