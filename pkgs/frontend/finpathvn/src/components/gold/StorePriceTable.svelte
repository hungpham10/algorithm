<script>
  import { onMount, onDestroy } from 'svelte';
  import { formatVN } from '$lib/utils.js';

  export let initialPrices = {};
  export let storeName = "";

  let items = [];
  let pollInterval;
  let loading = false;

  // Sorting state
  let sortColumn = "name"; // name, buy, sell, spread
  let sortDirection = "asc"; // asc, desc

  function parsePrices(data) {
    return Object.keys(data).map(productName => {
      const buyVal = Number(data[productName].buy || 0);
      const sellVal = Number(data[productName].sell || 0);

      // Chênh lệch mua - bán (Spread)
      const spread = Math.abs(sellVal - buyVal);

      return {
        name: productName,
        buy: buyVal,
        sell: sellVal,
        spread: spread
      };
    });
  }

  // Cập nhật danh sách khi initialPrices thay đổi
  $: {
    items = parsePrices(initialPrices);
    sortItems();
  }

  async function pollPrices() {
    if (!storeName) return;
    loading = true;
    try {
      const res = await fetch(`/api/investing/v2/stores/${encodeURIComponent(storeName)}/price`);
      if (res.ok) {
        const json = await res.json();
        const data = json.prices?.data || {};
        items = parsePrices(data);
        sortItems();
      }
    } catch (e) {
      console.error("Lỗi cập nhật giá vàng thực tế:", e);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    // Polling mỗi 30 giây để cập nhật giá
    pollInterval = setInterval(pollPrices, 30000);
  });

  onDestroy(() => {
    if (pollInterval) clearInterval(pollInterval);
  });

  function handleSort(column) {
    if (sortColumn === column) {
      sortDirection = sortDirection === "asc" ? "desc" : "asc";
    } else {
      sortColumn = column;
      sortDirection = column === "name" ? "asc" : "desc";
    }
    sortItems();
  }

  function sortItems() {
    items = [...items].sort((a, b) => {
      let valA = a[sortColumn];
      let valB = b[sortColumn];

      if (typeof valA === "string") {
        return sortDirection === "asc"
          ? valA.localeCompare(valB, "vi")
          : valB.localeCompare(valA, "vi");
      } else {
        return sortDirection === "asc"
          ? valA - valB
          : valB - valA;
      }
    });
  }
</script>

<div class="bg-white border border-slate-200/80 rounded-2xl overflow-hidden shadow-sm">
  <div class="px-5 py-4 bg-slate-50/70 border-b border-slate-100 flex items-center justify-between">
    <div class="flex items-center gap-2">
      <h3 class="font-bold text-slate-800 text-sm uppercase tracking-wider">
        Bảng giá các sản phẩm vàng hôm nay
      </h3>
      {#if loading}
        <span class="inline-flex h-2 w-2 rounded-full bg-amber-500 animate-ping"></span>
      {/if}
    </div>
    <span class="text-[10px] md:text-xs text-slate-400 font-medium whitespace-nowrap">
      Đơn vị: đồng
    </span>
  </div>

  <div class="overflow-x-auto">
    <table class="w-full border-collapse text-left text-xs md:text-sm">
      <thead>
        <tr class="bg-slate-50/30 text-slate-500 border-b border-slate-100 font-semibold select-none">

          <!-- Column: Product Name -->
          <th
            on:click={() => handleSort("name")}
            class="px-5 py-3 cursor-pointer hover:bg-slate-100/50 hover:text-slate-800 transition-colors w-[45%]"
          >
            <div class="flex items-center gap-1">
              SẢN PHẨM VÀNG
              <span class="text-[10px] text-slate-400">
                {#if sortColumn === "name"}
                  {sortDirection === "asc" ? "▲" : "▼"}
                {:else}
                  ↕
                {/if}
              </span>
            </div>
          </th>

          <!-- Column: Buy Price -->
          <th
            on:click={() => handleSort("buy")}
            class="px-5 py-3 cursor-pointer hover:bg-slate-100/50 hover:text-slate-800 transition-colors text-right w-[18%]"
          >
            <div class="flex items-center justify-end gap-1">
              MUA VÀO
              <span class="text-[10px] text-slate-400">
                {#if sortColumn === "buy"}
                  {sortDirection === "asc" ? "▲" : "▼"}
                {:else}
                  ↕
                {/if}
              </span>
            </div>
          </th>

          <!-- Column: Sell Price -->
          <th
            on:click={() => handleSort("sell")}
            class="px-5 py-3 cursor-pointer hover:bg-slate-100/50 hover:text-slate-800 transition-colors text-right w-[18%]"
          >
            <div class="flex items-center justify-end gap-1">
              BÁN RA
              <span class="text-[10px] text-slate-400">
                {#if sortColumn === "sell"}
                  {sortDirection === "asc" ? "▲" : "▼"}
                {:else}
                  ↕
                {/if}
              </span>
            </div>
          </th>

          <!-- Column: Spread -->
          <th
            on:click={() => handleSort("spread")}
            class="px-5 py-3 cursor-pointer hover:bg-slate-100/50 hover:text-slate-800 transition-colors text-right w-[19%]"
          >
            <div class="flex items-center justify-end gap-1">
              CHÊNH LỆCH
              <span class="text-[10px] text-slate-400">
                {#if sortColumn === "spread"}
                  {sortDirection === "asc" ? "▲" : "▼"}
                {:else}
                  ↕
                {/if}
              </span>
            </div>
          </th>

        </tr>
      </thead>

      <tbody class="divide-y divide-slate-100">
        {#if items.length > 0}
          {#each items as item}
            <tr class="hover:bg-slate-50/50 transition-colors group">

              <!-- Product Name -->
              <td class="px-5 py-3.5 align-middle">
                <span class="font-bold text-slate-800 group-hover:text-amber-600 transition-colors">
                  {item.name}
                </span>
              </td>

              <!-- Buy Price -->
              <td class="px-5 py-3.5 text-right font-mono font-semibold text-slate-700 align-middle tabular-nums">
                {formatVN(item.buy)}
              </td>

              <!-- Sell Price -->
              <td class="px-5 py-3.5 text-right font-mono font-bold text-sky-600 align-middle tabular-nums">
                {formatVN(item.sell)}
              </td>

              <!-- Spread -->
              <td class="px-5 py-3.5 text-right font-mono text-slate-400 align-middle tabular-nums">
                {formatVN(item.spread)}
              </td>

            </tr>
          {/each}
        {:else}
          <tr>
            <td colspan="4" class="px-5 py-12 text-center text-slate-400 italic">
              Không có dữ liệu giá sản phẩm vàng.
            </td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>
</div>
