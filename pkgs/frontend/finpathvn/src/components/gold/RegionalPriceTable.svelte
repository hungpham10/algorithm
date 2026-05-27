<script>
  import { onMount } from 'svelte';
  import { formatVN } from '$lib/utils.js';

  let selectedRegion = "Hồ Chí Minh";
  let items = [];
  let loading = false;

  // Cấu trúc phân trang theo Cursor Lịch Sử
  let currentPage = 1;
  let pageSize = 10;
  let nextCursor = null;

  /**
   * Mảng lưu vết các cursor để quay lại trang trước.
   * Trang 1: Không dùng cursor (null) -> Nằm ở index 0
   * Trang 2: Cần dùng cursor thu được từ trang 1 -> Nằm ở index 1
   * ...
   */
  let historyCursors = [null];

  const regions = ["Hồ Chí Minh", "Hà Nội", "Đà Nẵng", "Cần Thơ", "Hải Phòng", "Quảng Ninh"];

  // Hàm phụ trách reset trạng thái phân trang khi đổi khu vực
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
      if (cursor) {
        url += `&after=${cursor}`;
      }

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

  // Chuyển sang trang tiếp theo
  function handleNextPage() {
    if (!nextCursor || loading) return;

    // 1. Lưu lại cursor thu được để chuẩn bị cho trang mới
    historyCursors[currentPage] = nextCursor;

    // 2. Tăng số trang hiện tại lên
    currentPage += 1;

    // 3. Fetch dữ liệu trang mới bằng nextCursor vừa nhận
    fetchPricesByRegion(selectedRegion, nextCursor);
  }

  // Quay lại trang trước dựa vào mảng Lịch sử lưu trữ
  function handlePrevPage() {
    if (currentPage <= 1 || loading) return;

    // 1. Giảm số trang hiện tại xuống
    currentPage -= 1;

    // 2. Lấy lại chính xác cursor cũ của trang trước đó trong mảng history
    // Ví dụ: Đang ở trang 2 bấm lùi về trang 1 -> Lấy index 0 (chính là null)
    const targetCursor = historyCursors[currentPage - 1];

    // 3. Fetch lại dữ liệu
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
        if (data.region) {
          selectedRegion = data.region;
        }
        resetAndFetch(selectedRegion);
      } catch (e) {
        console.error(e);
        loading = false;
      }
    }, () => { loading = false; });
  }

  onMount(() => fetchPricesByRegion(selectedRegion, null));
</script>

<div class="w-full bg-white border border-gray-100 rounded-lg overflow-hidden">
  <div class="px-4 py-2.5 bg-[#f9fafb] flex flex-row justify-between items-center border-b border-gray-100">
    <div class="flex items-center gap-2">
      <div class="w-1 h-4 bg-yellow-400"></div>
      <h2 class="text-[11px] font-bold text-gray-500 uppercase tracking-wider">Vàng theo vùng miền</h2>
    </div>

    <div class="flex items-center gap-2">
      <button
        on:click={handleLocationClick}
        disabled={loading}
        class="p-1.5 bg-white hover:bg-gray-50 text-gray-400 rounded border border-gray-200 transition-colors flex items-center justify-center disabled:opacity-50"
        title="Định vị"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
        </svg>
      </button>

      <select
        value={selectedRegion}
        on:change={handleRegionChange}
        disabled={loading}
        class="bg-white border border-gray-200 text-gray-600 text-[11px] rounded px-2 py-1 outline-none appearance-none cursor-pointer min-w-[110px] disabled:opacity-50"
      >
        {#each regions as region}
          <option value={region}>{region}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="relative overflow-hidden">
    <table class="w-full border-collapse table-fixed">
      <thead class="bg-gray-50/30 text-[10px] uppercase text-gray-400 font-bold border-b border-gray-50">
        <tr>
          <th class="px-4 py-2 text-left w-[40%]">Sản phẩm</th>
          <th class="px-4 py-2 text-center w-[30%]">Mua vào</th>
          <th class="px-4 py-2 text-center w-[30%]">Bán ra</th>
        </tr>
      </thead>

      <tbody class="divide-y divide-gray-50">
        {#if loading}
          {#each Array(5) as _}
            <tr class="animate-pulse">
              <td class="px-4 py-4"><div class="h-3 bg-gray-100 rounded w-3/4"></div></td>
              <td class="px-4 py-4"><div class="h-3 bg-gray-100 rounded w-1/2 mx-auto"></div></td>
              <td class="px-4 py-4"><div class="h-3 bg-gray-100 rounded w-1/2 mx-auto"></div></td>
            </tr>
          {/each}
        {:else if items.length > 0}
          {#each items as item}
            <tr class="hover:bg-gray-50/50 transition-colors text-[13px] group">
              <td class="px-4 py-3.5 text-gray-600 font-medium truncate">
                <a
                  href={`/gia-vang/${item.store ? encodeURIComponent(item.store) : ''}`}
                  class="flex items-center gap-2 group-hover:text-blue-600 transition-colors"
                >
                  <span>{item.product}</span>
                  <svg xmlns="http://www.w3.org/2000/svg" class="h-3 w-3 opacity-0 group-hover:opacity-100 transition-opacity" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                    <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
                  </svg>
                </a>
              </td>
              <td class="px-4 py-3.5 text-center text-red-600 font-bold">{formatVN(item.buy)}</td>
              <td class="px-4 py-3.5 text-center text-blue-600 font-bold">{formatVN(item.sell)}</td>
            </tr>
          {/each}
        {:else}
          <tr>
            <td colspan="3" class="px-4 py-16 text-center">
              <p class="text-[11px] italic text-gray-400">Chưa có dữ liệu tại {selectedRegion}</p>
            </td>
          </tr>
        {/if}
      </tbody>
    </table>
  </div>

  {#if items.length > 0 || currentPage > 1}
  <div class="px-4 py-3 bg-gray-50/50 border-t border-gray-100 flex items-center justify-between">
    <div class="text-[11px] text-gray-400 font-medium">
      Trang {currentPage}
    </div>

    <div class="flex items-center gap-2">
      <button
        on:click={handlePrevPage}
        disabled={currentPage === 1 || loading}
        class="p-1 border border-gray-200 rounded bg-white hover:bg-gray-50 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </button>

      <div class="flex items-center justify-center min-w-8 h-7 border border-gray-200 rounded bg-gray-50 text-[11px] font-bold text-gray-600 select-none">
        {currentPage}
      </div>

      {#if nextCursor}
        <button
          on:click={handleNextPage}
          disabled={loading}
          class="p-1 border border-gray-200 rounded bg-white hover:bg-gray-50 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
        >
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
          </svg>
        </button>
      {/if}
    </div>
  </div>
  {/if}
</div>

<style>
  select {
    background-image: url("data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3E%3Cpath stroke='%239ca3af' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='m6 8 4 4 4-4'/%3E%3C/svg%3E");
    background-position: right 0.4rem center;
    background-repeat: no-repeat;
    background-size: 1em;
    padding-right: 1.5rem;
  }
</style>
