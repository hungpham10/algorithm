<script>
  import { onMount } from 'svelte';
  import { getGoldPricesByRegion } from '$lib/api.js';
  import { formatVN, getIP, removeVietnameseTones } from '$lib/utils.js';

  import PriceCell from './PriceCell.svelte';


  export let regions;

  let selectedRegion = "Hồ Chí Minh";
  let items = [];
  let loading = false;

  let currentPage = 1;
  let pageSize = 5;
  let nextCursor = null;
  let historyCursors = [null];

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
      const data = await getGoldPricesByRegion(region, cursor, pageSize);

      items = data?.items || [];
      nextCursor = data?.nextCursor || null;
    } catch (e) {
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
    loading = true;
    try {
      const userIp = await getIP();
      const res = await fetch(`/api/investing/v2/region/${userIp}`);

      if (!res.ok) {
        throw new Error("Lỗi fetch vùng miền từ IP");
      }

      const data = await res.json();

    if (data.query && data.query.length > 0) {
      const rawRegion = data.query[0];
      const cleanRaw = removeVietnameseTones(rawRegion).toLowerCase();
      const matchedRegion = regions.find(r =>
        removeVietnameseTones(r).toLowerCase() === cleanRaw
      );

      if (matchedRegion) {
        selectedRegion = matchedRegion;
        resetAndFetch(matchedRegion);
      } else {
        console.warn("Không tìm thấy tỉnh khớp trong danh sách:", rawRegion);
      }
    } else {
      console.error("Cấu trúc trả về không có region:", data);
    }
    } catch (e) {
      console.error("Lỗi định vị qua IP:", e);
    } finally {
      loading = false;
    }
  }

  // Hàm helper format hiển thị giá an toàn
  function displayPrice(val) {
    if (!val || val === 0 || val === "0") return "—";
    return formatVN(val);
  }

  onMount(() => fetchPricesByRegion(selectedRegion, null));
</script>

<section id="gold-by-province" class="max-w-[1200px] w-full mx-auto px-4 py-12">
  <div class="w-full bg-white border border-slate-200/80 shadow-sm rounded-2xl overflow-hidden">
    <div class="px-4 py-4 sm:px-5 bg-slate-50/70 flex flex-col sm:flex-row justify-between items-start sm:items-center gap-3 border-b border-slate-100">
      <div>
        <h2 class="text-xl sm:text-2xl md:text-3xl font-bold text-slate-800 tracking-tight">
          Giá vàng theo vùng miền
        </h2>
        <p class="text-[11px] sm:text-sm text-gray-500 mt-0.5 whitespace-nowrap">
          Đơn vị: triệu đồng/chỉ • Cập nhật <span class="current-date font-medium text-gray-700"></span>
        </p>
      </div>

      <div class="flex items-center gap-2 w-full sm:w-auto justify-end">
        <button
          on:click={handleLocationClick}
          disabled={loading}
          class="p-2 bg-white hover:bg-slate-50 text-slate-500 hover:text-slate-700 rounded-lg border border-slate-200 transition-all flex items-center justify-center disabled:opacity-50 active:scale-95 shadow-sm shrink-0"
          title="Định vị vị trí của bạn"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
            <circle cx="12" cy="11" r="2" stroke-width="2"/>
          </svg>
        </button>

        <div class="relative min-w-[130px]">
          <select
            value={selectedRegion}
            on:change={handleRegionChange}
            disabled={loading}
            class="w-full bg-white border border-slate-200 text-slate-700 text-xs font-medium rounded-lg pl-3 pr-8 py-2.5 outline-none appearance-none cursor-pointer disabled:opacity-50 shadow-sm transition-all hover:border-slate-300 focus:border-amber-500"
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
          <tr class="bg-slate-50/40 text-xs font-semibold border-b border-slate-100">
            <th class="px-4 md:px-6 py-3 text-left font-medium tracking-wide w-[40%] md:w-1/3">THƯƠNG HIỆU</th>
            <th class="px-4 md:px-6 py-3 text-right font-medium tracking-wide w-[30%] md:w-1/3">MUA VÀO</th>
            <th class="px-4 md:px-6 py-3 text-right font-medium tracking-wide w-[30%] md:w-1/3">BÁN RA</th>
          </tr>
        </thead>

        <tbody class="divide-y divide-slate-100">
          {#if loading}
            {#each Array(5) as _}
              <tr class="animate-pulse">
                <td class="px-4 md:px-6 py-4.5"><div class="h-4 bg-slate-100 rounded-md w-2/3"></div></td>
                <td class="px-4 md:px-6 py-4.5 flex justify-end"><div class="h-4 bg-slate-100 rounded-md w-16"></div></td>
                <td class="px-4 md:px-6 py-4.5"><div class="h-4 bg-slate-100 rounded-md w-16 ml-auto"></div></td>
              </tr>
            {/each}
          {:else if items.length > 0}
            {#each items as item}
              <tr class="hover:bg-slate-50/60 transition-colors group">
                <td class="p-2 md:p-4 pl-3 md:pl-6 align-middle w-[40%] md:w-1/3">
                  <div class="flex flex-col min-w-0">
                    <a
                      href={`/gia-vang/${item.store ? encodeURIComponent(item.store) : ''}`}
                      class="inline-flex items-start gap-1 hover:text-amber-600 font-semibold text-slate-800 transition-colors"
                    >
                      <span class="text-[14px] md:text-[17px] leading-tight break-words font-semibold text-gray-800 group-hover:text-amber-600 transition-colors">
                        {item.product}
                      </span>

                      <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5 mt-0.5 opacity-0 group-hover:opacity-100 text-amber-500 transition-all transform translate-x-[-4px] group-hover:translate-x-0 shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" />
                      </svg>
                    </a>

                    <span class="text-[10px] md:text-xs text-gray-400 mt-0.5 whitespace-nowrap">
                      Triệu đồng/chỉ
                    </span>
                  </div>
                </td>

                <td class="p-3 md:p-4 text-right tabular-nums align-middle text-[16px] md:text-[17px] font-semibold">
                  <PriceCell
                    enabled={false}
                    initialPrice={item.buy}
                    type="buy"
                  />
                </td>

                <td class="p-3 md:p-4 text-right tabular-nums align-middle text-[16px] md:text-[17px] font-bold text-sky-600">
                  <PriceCell
                    enabled={false}
                    initialPrice={item.sell}
                    type="sell"
                  />
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
        <div class="text-xs text-slate-400 font-medium"/>

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
</section>
