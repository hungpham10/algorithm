<script>
  import { onMount } from 'svelte';
  import { formatVN, formatDiff } from '$lib/utils.js';
  import { PRICE_EVENT_NAME } from '$lib/schema.js';

  export let index;
  export let type = "buy";
  export let initialPrice = "";
  export let initialDiff = "";

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
    const myUpdate = allUpdates.getItem(index);

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
    window.addEventListener(PRICE_EVENT_NAME, handleLiveUpdate);
    return () => {
      window.removeEventListener(PRICE_EVENT_NAME, handleLiveUpdate);
    };
  });
</script>

<div class="flex flex-col items-center justify-center leading-tight mx-auto">
  <div class="font-medium text-[18px] md:text-[18px] tracking-tight {priceColor}">
    {price}
  </div>
</div>
