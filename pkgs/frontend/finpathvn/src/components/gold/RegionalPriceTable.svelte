
<script>
  import { onMount } from 'svelte';
  import { formatVN } from '$lib/utils.js';

  let selectedRegion = "Hồ Chí Minh";
  let items = [];
  let loading = false;

  let currentPage = 1;
  let pageSize = 10;
  let nextCursor = null;
  let historyCursors = [null];

  const regions = ["Hồ Chí Minh", "Hà Nội", "Đà Nẵng", "Cần Thơ", "Hải Phòng", "Quảng Ninh"];

  function resetAndFetch(region) {
    currentPage = 1;
    nextCursor = null;
    historyCursors = [null];
    fetchPricesByRegion(region, null);
  }

  function handleRegionChange(e) {
    selectedRegion = e.target.value;
    resetAndFetch(selectedRegion);
  }

  async function fetchPricesByRegion(region, cursor = null) {
    loading = true;
    try {
      let url = `/api/investing/v2/prices/by-province/${encodeURIComponent(region)}?limit=${pageSize}&degree=1000000`;
      if (cursor) url += `&after=${cursor}`;

      const response = await fetch(url);
      if (response.ok) {
        const rawData = await response.json();
        let flattenedItems = [];

        // Map cấu trúc Object lồng nhau của API thành mảng phẳng
        if (Array.isArray(rawData)) {
          const priceData = rawData[0].prices.data;

          flattenedItems = Object.keys(priceData).map(brandName => {
            return {
              product: brandName,
              buy: priceData[brandName].buy,
              sell: priceData[brandName].sell,
              store: brandName
            };
          });

          // Lấy next cursor từ cấu trúc `prices.next_after`
          nextCursor = rawData[0].prices.next_after || null;
        } else {
          nextCursor = null;
        }
        items = flattenedItems;
      } else {
        items = [];
        nextCursor = null;
      }
    } catch (e) {
      console.error("Lỗi fetch:", e);
      items = [];
      nextCursor = null;
    } finally {
      loading = false;
    }
  }

  function handleNextPage() {
    if (!nextCursor || loading) return;
    historyCursors[currentPage] = nextCursor;
    currentPage += 1;
    fetchPricesByRegion(selectedRegion, nextCursor);
  }

  function handlePrevPage() {
    if (currentPage <= 1 || loading) return;
    currentPage -= 1;
    const targetCursor = historyCursors[currentPage - 1];
    fetchPricesByRegion(selectedRegion, targetCursor);
  }

  async function handleLocationClick() {
    if (!navigator.geolocation) return;
    loading = true;
    navigator.geolocation.getCurrentPosition(async (position) => {
      const { latitude, longitude } = position.coords;
      try {
        const res = await fetch(`/api/investing/v2/geo-to-region?lat=${latitude}&lng=${longitude}`);
        const data = await res.json();
        if (data.region) selectedRegion = data.region;
        resetAndFetch(selectedRegion);
      } catch (e) {
        console.error(e);
        loading = false;
      }
    }, () => { loading = false; });
  }

  // Hàm helper format hiển thị giá an toàn
  function displayPrice(val) {
    if (!val || val === 0 || val === "0") return "—";
    return formatVN(val);
  }

  onMount(() => fetchPricesByRegion(selectedRegion, null));
</script>

<div class="w-full bg-white border border-slate-200/80 shadow-sm rounded-2xl overflow-hidden">
  <div class="px-5 py-4 bg-slate-50/70 flex flex-row justify-between items-center border-b border-slate-100">
    <div class="flex items-center gap-2.5">
      <div class="w-1 h-4 bg-amber-500 rounded-full"></div>
      <h2 class="text-xl font-semibold text-slate-800 tracking-tight">Giá vàng theo vùng miền</h2>
    </div>

    <div class="flex items-center gap-2">
      <button
        on:click={handleLocationClick}
        disabled={loading}
        class="p-2 bg-white hover:bg-slate-50 text-slate-500 hover:text-slate-700 rounded-lg border border-slate-200 transition-all flex items-center justify-center disabled:opacity-50 active:scale-95 shadow-sm"
        title="Định vị vị trí của bạn"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
          <circle cx="12" cy="11" r="2" stroke-width="2"/>
        </svg>
      </button>

      <div class="relative">
        <select
          value={selectedRegion}
          on:change={handleRegionChange}
          disabled={loading}
          class="bg-white border border-slate-200 text-slate-700 text-xs font-medium rounded-lg pl-3 pr-8 py-2 outline-none appearance-none cursor-pointer min-w-[130px] disabled:opacity-50 shadow-sm transition-all hover:border-slate-300 focus:border-amber-500"
        >
          {#each regions as region}
            <option value={region}>{region}</option>
          {/each}
        </select>
        <div class="absolute inset-y-0 right-2.5 flex items-center pointer-events-none text-slate-400">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
    </div>
  </div>

  <div class="relative overflow-x-auto">
    <table class="w-full border-collapse">
      <thead>
        <tr class="bg-slate-50/40 text-xs font-semibold text-slate-400 border-b border-slate-100">
          <th class="px-6 py-3 text-left font-medium tracking-wide w-[45%]">Thương hiệu / Sản phẩm</th>
          <th class="px-6 py-3 text-right font-medium tracking-wide w-[27.5%]">Mua vào (đ/lượng)</th>
          <th class="px-6 py-3 text-right font-medium tracking-wide w-[27.5%]">Bán ra (đ/lượng)</th>
        </tr>
      </thead>

      <tbody class="divide-y divide-slate-100">
        {#if loading}
          {#each Array(5) as _}
            <tr class="animate-pulse">
              <td class="px-6 py-4.5"><div class="h-4 bg-slate-100 rounded-md w-2/3"></div></td>
              <td class="px-6 py-4.5 flex justify-end"><div class="h-4 bg-slate-100 rounded-md w-16"></div></td>
              <td class="px-6 py-4.5"><div class="h-4 bg-slate-100 rounded-md w-16 ml-auto"></div></td>
            </tr>
          {/each}
        {:else if items.length > 0}
          {#each items as item}
            <tr class="hover:bg-slate-50/60 transition-colors group">
              <td class="px-6 py-4 text-slate-700 font-medium text-sm max-w- truncate">
                <a
                  href={`/gia-vang/${item.store ? encodeURIComponent(item.store) : ''}`}
                  class="inline-flex items-center gap-1.5 hover:text-amber-600 font-semibold text-slate-800 transition-colors"
                >
                  <span class="truncate">{item.product}</span>
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 opacity-0 group-hover:opacity-100 text-amber-500 transition-all transform translate-x-[-4px] group-hover:translate-x-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" />
                  </svg>
                </a>
              </td>

              <td class="px-6 py-4 text-right tabular-nums">
                <span class="inline-block px-2.5 py-1 text-sm font-bold text-rose-600 rounded-md bg-rose-50/40">
                  {displayPrice(item.buy)}
                </span>
              </td>

              <td class="px-6 py-4 text-right tabular-nums">
                <span class="inline-block px-2.5 py-1 text-sm font-bold text-emerald-600 rounded-md bg-emerald-50/40">
                  {displayPrice(item.sell)}
                </span>
              </td>
            </tr>
          {/each}
        {:else}
          <tr>
            <td colspan="3" class="px-6 py-16 text-center">
              <div class="flex flex-col items-center justify-center gap-2 text-slate-400">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-8 w-8 text-slate-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M20 13V6a2 2 0 00-2-2H6a2 2 0 00-2 2v7m16 0a2 2 0 01-2 2H6a2 2 0 01-2-2m16 0h-2.586a1 1 0 00-.707.293l-2.414 2.414a1 1 0 01-.707.293h-3.172a1 1 0 01-.707-.293l-2.414-2.414A1 1 0 006.586 13H4" />
                </svg>
                <p class="text-xs italic">Không tìm thấy dữ liệu giá vàng tại khu vực {selectedRegion}</p>
              </div>
            </td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  {#if items.length > 0 || currentPage > 1}
    <div class="px-6 py-3.5 bg-slate-50/50 border-t border-slate-100 flex items-center justify-between">
      <div class="text-xs text-slate-400 font-medium">
        Đang hiển thị trang <span class="text-slate-700 font-semibold">{currentPage}</span>
      </div>

      <div class="flex items-center gap-1.5">
        <button
          on:click={handlePrevPage}
          disabled={currentPage === 1 || loading}
          class="p-1.5 border border-slate-200 rounded-lg bg-white text-slate-500 hover:bg-slate-50 hover:text-slate-700 disabled:opacity-30 disabled:cursor-not-allowed transition-all shadow-sm"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
          </svg>
        </button>

        <div class="flex items-center justify-center px-3 h-7 border border-slate-200 rounded-lg bg-white text-xs font-semibold text-slate-700 shadow-sm min-w-[2rem]">
          {currentPage}
        </div>

        {#if nextCursor}
          <button
            on:click={handleNextPage}
            disabled={loading}
            class="p-1.5 border border-slate-200 rounded-lg bg-white text-slate-500 hover:bg-slate-50 hover:text-slate-700 disabled:opacity-30 disabled:cursor-not-allowed transition-all shadow-sm"
          >
            <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
            </svg>
          </button>
        {/if}
      </div>
    </div>
  {/if}
</div>
