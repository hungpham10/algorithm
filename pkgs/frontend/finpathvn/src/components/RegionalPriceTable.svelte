
<script>
  import { onMount } from 'svelte';
  export let formatVN;

  let selectedRegion = "Hồ Chí Minh";
  let items = [];
  let loading = false;

  // Các biến phân trang
  let currentPage = 1;
  let totalPages = 1; // Giả sử BE trả về hoặc mặc định
  let pageSize = 10;

  const regions = ["Hồ Chí Minh", "Hà Nội", "Đà Nẵng", "Cần Thơ", "Hải Phòng", "Quảng Ninh"];

  async function fetchPricesByRegion(region, page = 1) {
    loading = true;
    try {
      // Thêm tham số page và limit vào API (tùy chỉnh theo BE của bạn)
      const response = await fetch(`/api/investing/v2/prices/by-provice?name=${encodeURIComponent(region)}&page=${page}&limit=${pageSize}`);
      if (response.ok) {
        const data = await response.json();
        // Giả sử data có cấu trúc { items: [], totalPages: 5 } hoặc chỉ là mảng
        if (Array.isArray(data)) {
          items = data;
          // Nếu BE không trả về totalPages, bạn có thể tạm tính hoặc ẩn nút Next nếu mảng rỗng
        } else {
          items = data.items || [];
          totalPages = data.totalPages || 1;
        }
      } else {
        items = [];
      }
    } catch (e) {
      console.error("Lỗi:", e);
      items = [];
    } finally {
      loading = false;
    }
  }

  function goToPage(p) {
    if (p < 1 || (totalPages > 1 && p > totalPages)) return;
    currentPage = p;
    fetchPricesByRegion(selectedRegion, currentPage);
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
          currentPage = 1; // Reset về trang 1 khi đổi vùng
        }
      } catch (e) {
        console.error(e);
      } finally {
        loading = false;
      }
    }, () => { loading = false; });
  }

  // Theo dõi khi đổi vùng thì reset trang và fetch lại
  $: if (selectedRegion) {
    currentPage = 1;
    fetchPricesByRegion(selectedRegion, 1);
  }

  onMount(() => fetchPricesByRegion(selectedRegion, currentPage));
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
        class="p-1.5 bg-white hover:bg-gray-50 text-gray-400 rounded border border-gray-200 transition-colors flex items-center justify-center"
        title="Định vị"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
        </svg>
      </button>

      <select
        bind:value={selectedRegion}
        class="bg-white border border-gray-200 text-gray-600 text-[11px] rounded px-2 py-1 outline-none appearance-none cursor-pointer min-w-[110px]"
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
            <tr class="hover:bg-gray-50/50 transition-colors text-[13px]">
              <td class="px-4 py-3.5 text-gray-600 font-medium truncate">{item.product}</td>
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
        on:click={() => goToPage(currentPage - 1)}
        disabled={currentPage === 1 || loading}
        class="p-1 border border-gray-200 rounded bg-white hover:bg-gray-50 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M15 19l-7-7 7-7" />
        </svg>
      </button>

      <div class="flex items-center gap-1">
        <input
          type="number"
          bind:value={currentPage}
          on:change={(e) => goToPage(parseInt(e.target.value))}
          class="w-10 h-7 border border-gray-200 rounded text-center text-[11px] font-bold text-gray-600 outline-none focus:border-yellow-400"
          min="1"
        />
      </div>

      <button
        on:click={() => goToPage(currentPage + 1)}
        disabled={items.length < pageSize || loading}
        class="p-1 border border-gray-200 rounded bg-white hover:bg-gray-50 disabled:opacity-30 disabled:cursor-not-allowed transition-all"
      >
        <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4 text-gray-500" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 5l7 7-7 7" />
        </svg>
      </button>
    </div>
  </div>
  {/if}
</div>

<style>
  /* Chrome, Safari, Edge, Opera: Ẩn mũi tên input number */
  input::-webkit-outer-spin-button,
  input::-webkit-inner-spin-button {
    -webkit-appearance: none;
    margin: 0;
  }

  select {
    background-image: url("data:image/svg+xml;charset=utf-8,%3Csvg xmlns='http://www.w3.org/2000/svg' fill='none' viewBox='0 0 20 20'%3E%3Cpath stroke='%239ca3af' stroke-linecap='round' stroke-linejoin='round' stroke-width='1.5' d='m6 8 4 4 4-4'/%3E%3C/svg%3E");
    background-position: right 0.4rem center;
    background-repeat: no-repeat;
    background-size: 1em;
    padding-right: 1.5rem;
  }
</style>
