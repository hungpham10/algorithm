<script>
  import { onMount } from 'svelte';
  import { getStoreMeta } from '$lib/store-meta.js';

  export let initialStores = [];
  export let allProvinces = [];

  let searchQuery = "";
  let selectedProvince = "";
  let filteredStores = [];

  // Khởi tạo và lọc danh sách cửa hàng khi tìm kiếm hoặc thay đổi tỉnh thành
  $: {
    filteredStores = initialStores.filter(store => {
      const matchesSearch = store.name.toLowerCase().includes(searchQuery.toLowerCase());

      // Kiểm tra xem tiệm vàng có hoạt động ở tỉnh thành được chọn không
      const matchesProvince = selectedProvince === "" ||
        store.provinces.some(p => p.toLowerCase() === selectedProvince.toLowerCase());

      return matchesSearch && matchesProvince;
    });
  }

  function clearFilters() {
    searchQuery = "";
    selectedProvince = "";
  }
</script>

<!-- Search and Filter Bar -->
<div class="mb-10 bg-slate-50 border border-slate-100 rounded-3xl p-6 md:p-8">
  <div class="grid md:grid-cols-3 gap-4 items-end">
    <!-- Search Input -->
    <div class="md:col-span-2 relative">
      <label for="search" class="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-2">Tìm kiếm tiệm vàng</label>
      <div class="relative">
        <input
          id="search"
          type="text"
          bind:value={searchQuery}
          placeholder="Nhập tên tiệm vàng (ví dụ: DOJI, SJC, Ancarat...)"
          class="w-full bg-white border border-slate-200 text-slate-800 rounded-2xl pl-11 pr-4 py-3.5 outline-none transition-all shadow-sm focus:border-amber-500 focus:ring-1 focus:ring-amber-500 text-sm font-medium"
        />
        <div class="absolute inset-y-0 left-4 flex items-center pointer-events-none text-slate-400">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        </div>
      </div>
    </div>

    <!-- Province Filter -->
    <div class="relative">
      <label for="province-filter" class="block text-xs font-semibold uppercase tracking-wider text-slate-400 mb-2">Lọc theo khu vực</label>
      <div class="relative">
        <select
          id="province-filter"
          bind:value={selectedProvince}
          class="w-full bg-white border border-slate-200 text-slate-700 rounded-2xl pl-4 pr-10 py-3.5 outline-none appearance-none cursor-pointer shadow-sm transition-all focus:border-amber-500 text-sm font-semibold"
        >
          <option value="">Tất cả tỉnh thành ({allProvinces.length})</option>
          {#each allProvinces as province}
            <option value={province}>{province}</option>
          {/each}
        </select>
        <div class="absolute inset-y-0 right-4 flex items-center pointer-events-none text-slate-400">
          <svg xmlns="http://www.w3.org/2000/svg" class="h-4 w-4" fill="none" viewBox="0 0 24 24" stroke="currentColor">
            <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M19 9l-7 7-7-7" />
          </svg>
        </div>
      </div>
    </div>
  </div>

  {#if searchQuery !== "" || selectedProvince !== ""}
    <div class="mt-4 flex items-center justify-between text-xs text-slate-500">
      <div>
        Tìm thấy <strong>{filteredStores.length}</strong> kết quả phù hợp.
      </div>
      <button
        on:click={clearFilters}
        class="text-amber-600 hover:text-amber-700 font-bold flex items-center gap-1 transition-colors hover:underline"
      >
        Xóa bộ lọc
        <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    </div>
  {/if}
</div>

<!-- Stores Grid List -->
{#if filteredStores.length > 0}
  <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
    {#each filteredStores as store (store.id)}
      {@const meta = getStoreMeta(store.name)}
      <div class="group relative bg-white border border-slate-100 rounded-3xl p-6 transition-all duration-300 hover:-translate-y-1 hover:shadow-xl hover:border-amber-200/50 flex flex-col justify-between">

        <!-- Top decoration line with brand color -->
        <div class="absolute top-0 left-0 right-0 h-1.5 rounded-t-3xl bg-gradient-to-r {meta.theme.gradient}"></div>

        <div>
          <!-- Badge and Logo Header -->
          <div class="flex items-center justify-between gap-4 mb-4">
            <span class="px-2.5 py-1 rounded-full text-[10px] font-bold tracking-wide uppercase {meta.theme.bgLight} {meta.theme.text} border {meta.theme.border}">
              {meta.badge}
            </span>
            <div class="flex items-center gap-1.5 text-xs text-slate-400 font-semibold">
              <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z" />
                <circle cx="12" cy="11" r="2" stroke-width="2"/>
              </svg>
              <span>{store.provinces.length} khu vực</span>
            </div>
          </div>

          <!-- Store Name -->
          <h3 class="text-lg font-bold text-slate-800 group-hover:text-amber-600 transition-colors line-clamp-1 mb-2" title={store.name}>
            {meta.shortName}
          </h3>

          <p class="text-xs text-slate-500 line-clamp-2 leading-relaxed mb-4">
            {meta.description}
          </p>

          <!-- Branch Provinces List -->
          {#if store.provinces.length > 0}
            <div class="flex flex-wrap gap-1.5 mb-6">
              {#each store.provinces.slice(0, 4) as province}
                <span class="px-2 py-0.5 rounded-md bg-slate-50 text-[10px] text-slate-500 font-medium border border-slate-100/50">
                  {province}
                </span>
              {/each}
              {#if store.provinces.length > 4}
                <span class="px-2 py-0.5 rounded-md bg-slate-50 text-[10px] text-slate-400 font-bold border border-slate-100/50">
                  +{store.provinces.length - 4}
                </span>
              {/if}
            </div>
          {:else}
            <div class="h-6 mb-6 flex items-center text-[10px] italic text-slate-400">
              Đang cập nhật địa bàn hoạt động...
            </div>
          {/if}
        </div>

        <!-- Action Button and Contacts -->
        <div class="pt-4 border-t border-slate-50 flex items-center justify-between gap-4">
          <div class="flex flex-col gap-0.5">
            {#if meta.phone}
              <span class="text-[10px] text-slate-400 font-medium">Hotline:</span>
              <a href="tel:{meta.phone.replace(/\s+/g, '')}" class="text-[11px] font-bold text-slate-700 hover:text-amber-600 transition-colors">
                {meta.phone}
              </a>
            {:else}
              <span class="text-[10px] text-slate-400 font-medium">Hotline:</span>
              <span class="text-[11px] font-bold text-slate-400 italic">Đang cập nhật</span>
            {/if}
          </div>

          <a
            href={`/gia-vang/${encodeURIComponent(store.name)}`}
            class="inline-flex items-center gap-1 bg-slate-900 hover:bg-amber-600 text-white font-semibold text-xs px-3.5 py-2 rounded-xl transition-all shadow-sm active:scale-95 shrink-0"
          >
            Chi tiết
            <svg xmlns="http://www.w3.org/2000/svg" class="h-3.5 w-3.5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
              <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2.5" d="M9 5l7 7-7 7" />
            </svg>
          </a>
        </div>

      </div>
    {/each}
  </div>
{:else}
  <div class="text-center py-20 bg-slate-50 border border-dashed border-slate-200 rounded-3xl">
    <div class="max-w-xs mx-auto flex flex-col items-center gap-3 text-slate-400">
      <svg xmlns="http://www.w3.org/2000/svg" class="h-12 w-12 text-slate-300" fill="none" viewBox="0 0 24 24" stroke="currentColor">
        <path stroke-linecap="round" stroke-linejoin="round" stroke-width="1.5" d="M19.428 15.428a2 2 0 00-1.022-.547l-2.387-.477a6 6 0 00-3.86.517l-.318.158a6 6 0 01-3.86.517L6.05 15.21a2 2 0 00-1.806.547M8 4h8l-1 1v5.172a2 2 0 00.586 1.414l5 5c1.26 1.26.367 3.414-1.415 3.414H4.828c-1.782 0-2.674-2.154-1.414-3.414l5-5A2 2 0 009 10.172V5L8 4z" />
      </svg>
      <h4 class="text-sm font-bold text-slate-700">Không tìm thấy tiệm vàng</h4>
      <p class="text-xs text-slate-500 leading-relaxed">Không tìm thấy tiệm vàng nào phù hợp với bộ lọc tìm kiếm của bạn. Vui lòng thử từ khóa khác.</p>
      <button
        on:click={clearFilters}
        class="mt-2 text-xs font-bold text-amber-600 hover:text-amber-700 px-4 py-2 bg-white rounded-xl border border-slate-200 hover:border-slate-300 transition-all shadow-sm active:scale-95"
      >
        Làm mới bộ lọc
      </button>
    </div>
  </div>
{/if}
