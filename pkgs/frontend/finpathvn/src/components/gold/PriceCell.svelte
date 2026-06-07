<script>
  import { onMount } from 'svelte';
  import { formatVN, formatDiff } from '$lib/utils.js';
  import { PRICE_EVENT_NAME } from '$lib/schema.js';

  export let index;
  export let enabled = false;
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
        price = formatVN(type === "buy" ? myUpdate.buy : myUpdate.sell);
	      diff = formatDiff(type === "buy"? myUpdate.diffBuy: myUpdate.diffSell);
      }
    }
  }

  onMount(() => {
    if (enabled) {
      window.addEventListener(PRICE_EVENT_NAME, handleLiveUpdate);
      return () => {
        window.removeEventListener(PRICE_EVENT_NAME, handleLiveUpdate);
      };
    }

    return () => {};
  });
</script>

<div class="flex flex-col items-right justify-center leading-tight mx-auto">
  <div class="font-medium text-[16px] md:text-[17px] tracking-tight {priceColor}">
    {price}
  </div>
</div>
