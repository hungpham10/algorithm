<script>
  import { onMount } from 'svelte';
  import { getExchangeRatesWithHistory } from '$lib/api.js';

  let exchangeData = [];
  let currentDate = "";
  let isLoading = true;
  let isRefreshing = false; // Trạng thái riêng khi bấm nút làm mới
  let focusedCurrency = null;

  // --- BỘ NHỚ LƯU TRỮ LỊCH SỬ 30 NGÀY ---
  let historyCache = {};
  let currentTrendPoints = "";

  // --- CẤU HÌNH PHÂN TRANG ---
  let currentPage = 1;
  const itemsPerPage = 6;
  let totalPages = 1;
  let pagedData = [];

  const currencyNames = {
    USD: "Đô la Mỹ", EUR: "Euro", GBP: "Bảng Anh", JPY: "Yên Nhật",
    AUD: "Đô la Úc", SGD: "Đô la Singapore", THB: "Baht Thái Lan",
    CAD: "Đô la Canada", CHF: "Franc Thụy Sĩ", HKD: "Đô la Hồng Kông",
    CNY: "Nhân dân tệ", DKK: "Krone Đan Mạch", INR: "Rupee Ấn Độ",
    KRW: "Won Hàn Quốc", KWD: "Dinar Kuwait", MYR: "Ringgit Malaysia",
    NOK: "Krone Na Uy", RUB: "Rúp Nga", SAR: "Riyal Ả Rập Xê Út", SEK: "Krona Thụy Điển"
  };

  function formatMoney(value) {
    const num = parseFloat(value);
    if (isNaN(num)) return value;
    return num.toLocaleString('vi-VN', { minimumFractionDigits: 2, maximumFractionDigits: 2 });
  }

  function updatePagedData() {
    totalPages = Math.ceil(exchangeData.length / itemsPerPage);
    const startIndex = (currentPage - 1) * itemsPerPage;
    const endIndex = startIndex + itemsPerPage;
    pagedData = exchangeData.slice(startIndex, endIndex);
  }

  function generateTrendPoints(currencyCode) {
    const history = historyCache[currencyCode];
    if (!history || history.length < 2) {
      currentTrendPoints = "";
      return;
    }

    const values = history.map(h => h.cash);
    const min = Math.min(...values);
    const max = Math.max(...values);
    const range = max - min === 0 ? 1 : max - min;

    const width = 300;
    const height = 70;
    const padding = 5;

    const points = history.map((item, index) => {
      const x = (index / (history.length - 1)) * width;
      const y = height - padding - ((item.cash - min) / range) * (height - 2 * padding);
      return `${x},${y}`;
    });

    currentTrendPoints = `M ${points.join(' L ')}`;
  }

  async function fetchExchangeRates() {
    const data = await getExchangeRatesWithHistory();

    exchangeData = data.exchangeData;
    historyCache = data.historyCache;
    currentDate = data.currentDate;

    updatePagedData();

    // Giữ tiêu điểm cũ hoặc thiết lập mặc định USD
    const currentCode = focusedCurrency ? focusedCurrency.code : "USD";
    focusedCurrency = exchangeData.find(i => i.code === currentCode) || exchangeData;

    if (focusedCurrency) {
      generateTrendPoints(focusedCurrency.code);
    }
  }

  // Khởi chạy lần đầu khi mount component
  onMount(async () => {
    await fetchExchangeRates();
    isLoading = false;
  });

  // Xử lý sự kiện khi bấm nút Làm mới bằng tay
  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;
    await fetchExchangeRates();
    isRefreshing = false;
  }

  function handleSelectCurrency(currency) {
    focusedCurrency = currency;
    generateTrendPoints(currency.code);
  }

  function goToPage(page) {
    if (page >= 1 && page <= totalPages) {
      currentPage = page;
      updatePagedData();
    }
  }
</script>

<section id="exchange-rates" class="max-w-[1200px] w-full mx-auto px-4 py-3">
  <header class="p-6 border-gray-100 bg-white">
    <div class="flex flex-col sm:flex-row sm:items-center justify-between mb-8 gap-4">
      <div>
        <div class="flex items-center gap-2">
          <h2 class="text-3xl font-bold text-gray-900 tracking-tight">
            Tỷ giá ngoại tệ Vietcombank
          </h2>
        </div>
        <p class="text-sm text-gray-900 mt-1 py-3">
          Dữ liệu mua bán ngoại tệ kèm xu hướng thị trường 30 ngày qua bộ lọc thực tế
        </p>
      </div>

      <div class="flex items-center gap-3 self-start sm:self-auto">
        <div class="text-xs text-gray-900 bg-gray-100/80 px-3 py-1.5 rounded-md">
          Cập nhật: {currentDate || 'Đang tải...'}
        </div>

        <button
          on:click={handleRefresh}
          disabled={isLoading || isRefreshing}
          class="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-md bg-white border border-gray-200 text-xs font-semibold text-gray-600 hover:bg-gray-50 active:bg-gray-100/70 disabled:opacity-50 disabled:cursor-not-allowed shadow-sm transition-all"
          title="Cập nhật giá mới"
        >
          <svg
            xmlns="http://www.w3.org/2000/svg"
            class="w-3.5 h-3.5 text-gray-500 {isRefreshing ? 'animate-spin text-blue-600' : ''}"
            fill="none"
            viewBox="0 0 24 24"
            stroke="currentColor"
            stroke-width="2.5"
          >
            <path stroke-linecap="round" stroke-linejoin="round" d="M4 4v5h.582m15.356 2A8.001 8.001 0 1121.253 8H18" />
          </svg>
          <span>{isRefreshing ? 'Đang làm mới...' : 'Làm mới'}</span>
        </button>
      </div>
    </div>
  </header>

  {#if isLoading}
    <div class="flex justify-center items-center py-20 text-gray-900 gap-2">
      <div class="w-5 h-5 border-2 border-blue-600 border-t-transparent rounded-full animate-spin"></div>
      <span>Đang cấu hình dữ liệu và phân tích xu hướng 30 ngày...</span>
    </div>
  {:else}
    <div class="grid grid-cols-1 lg:grid-cols-12 gap-6 items-start">

      <div class="lg:col-span-5 sticky top-3">
        {#if focusedCurrency}
          <div class="bg-white rounded-3xl border border-gray-100 p-6 shadow-[0_8px_30px_rgba(0,0,0,0.02)] transition-all">

            <div class="flex items-center justify-between mb-4">
              <span class="text-xs font-bold uppercase tracking-wider text-blue-600 bg-blue-50 px-2.5 py-1 rounded-md">
                XU HƯỚNG 30 NGÀY
              </span>
              <div class="w-8 h-8 rounded-full bg-blue-50 text-blue-500 flex items-center justify-center">
                <svg xmlns="http://www.w3.org/2000/svg" class="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M13 7h8m0 0v8m0-8l-8 8-4-4-6 6" />
                </svg>
              </div>
            </div>

            <h3 class="text-2xl font-bold text-gray-800 tracking-tight">
              {focusedCurrency.name} · {focusedCurrency.code}/VND
            </h3>

            <div class="flex items-baseline gap-8 mt-5 mb-6 flex-wrap">
              <div class="flex flex-col">
                <span class="text-[11px] text-gray-900 font-medium uppercase tracking-wider">MUA VÀO</span>
                <span class="text-2xl md:text-3xl font-extrabold text-gray-900 tracking-tighter mt-1">
                  {focusedCurrency.cash}
                </span>
              </div>

              <div class="flex flex-col">
                <span class="text-[11px] text-gray-900 font-medium uppercase tracking-wider">BÁN RA</span>
                <span class="text-2xl md:text-3xl font-extrabold text-sky-600 tracking-tighter mt-1">
                  {focusedCurrency.sell}
                </span>
              </div>
            </div>

            <div class="w-full bg-gray-50/70 rounded-2xl border border-gray-100 p-4 flex flex-col justify-between">
              <div class="flex justify-between items-center mb-2">
                <span class="text-[11px] text-gray-900 font-semibold uppercase tracking-wider">Biểu đồ mua tiền mặt (30 ngày)</span>
              </div>

              <div class="h-[80px] w-full flex items-center justify-center pt-2">
                {#if currentTrendPoints}
                  <svg class="w-full h-full overflow-visible" viewBox="0 0 300 70" preserveAspectRatio="none">
                    <line x1="0" y1="65" x2="300" y2="65" stroke="#f1f5f9" stroke-width="1" />
                    <line x1="0" y1="5" x2="300" y2="5" stroke="#f1f5f9" stroke-width="1" />

                    <path
                      d={currentTrendPoints}
                      fill="none"
                      stroke="#2563eb"
                      stroke-width="2.5"
                      stroke-linecap="round"
                      stroke-linejoin="round"
                    />
                  </svg>
                {:else}
                  <span class="text-xs text-gray-900 italic">Không có dữ liệu lịch sử</span>
                {/if}
              </div>

              <div class="flex justify-between text-[10px] text-gray-900 mt-2 font-medium">
                <span>30 ngày trước</span>
                <span>Hôm nay</span>
              </div>
            </div>

          </div>
        {/if}
      </div>

      <div class="lg:col-span-7 bg-white rounded-3xl border border-gray-100 shadow-[0_8px_30px_rgba(0,0,0,0.02)] overflow-hidden">
        <div class="overflow-x-auto">
          <table class="w-full text-left border-collapse">
            <thead>
              <tr class="bg-gray-50/70 border-b border-gray-100 text-xs font-bold uppercase tracking-wider text-gray-500">
                <th class="py-4 px-6">NGOẠI TỆ</th>
                <th class="py-4 px-4 text-right text-gray-800">MUA VÀO</th>
                <th class="py-4 px-6 text-right text-sky-600">BÁN RA</th>
              </tr>
            </thead>
            <tbody class="text-sm divide-y divide-gray-50">
              {#each pagedData as item}
                <tr
                  class="hover:bg-blue-50/30 transition-colors cursor-pointer group
                         {focusedCurrency?.code === item.code ? 'bg-blue-50/40' : ''}"
                  on:click={() => handleSelectCurrency(item)}
                >
                  <td class="py-4 px-6 flex items-center gap-3">
                    <div class="w-8 h-8 rounded-full bg-gray-100 text-gray-700 font-bold text-xs flex items-center justify-center shrink-0
                                group-hover:bg-blue-100 group-hover:text-blue-700 transition-colors
                                {focusedCurrency?.code === item.code ? 'bg-blue-100 text-blue-700' : ''}">
                      {item.code.substring(0, 2)}
                    </div>
                    <div class="flex flex-col">
                      <span class="font-bold text-gray-800 text-[15px] group-hover:text-blue-600 transition-colors
                                   {focusedCurrency?.code === item.code ? 'text-blue-600' : ''}">
                        {item.code}
                      </span>
                      <span class="text-xs text-gray-900">{item.name}</span>
                    </div>
                  </td>

                  <td class="py-4 px-4 text-right font-semibold text-gray-900 text-[15px] md:text-[16px]">
                    {item.cash}
                  </td>

                  <td class="py-4 px-6 text-right font-bold text-sky-600 text-[15px] md:text-[16px]">
                    {item.sell}
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
        </div>

        {#if totalPages > 1}
          <div class="flex items-center justify-between px-6 py-4 border-t border-gray-100 bg-gray-50/30 text-xs">
            <span class="text-gray-500">
              Hiển thị trang <strong class="text-gray-700">{currentPage}</strong> trên <strong class="text-gray-700">{totalPages}</strong> trang
            </span>
            <div class="flex items-center gap-1">
              <button
                class="px-3 py-1.5 rounded-lg border border-gray-200 bg-white font-medium text-gray-600 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                on:click={() => goToPage(currentPage - 1)}
                disabled={currentPage === 1}
              >
                Trước
              </button>

              {#each Array(totalPages) as _, index}
                <button
                  class="w-8 h-8 rounded-lg font-medium transition-colors
                         {currentPage === index + 1
                           ? 'bg-blue-600 text-white'
                           : 'border border-gray-200 bg-white text-gray-600 hover:bg-gray-50'}"
                  on:click={() => goToPage(index + 1)}
                >
                  {index + 1}
                </button>
              {/each}

              <button
                class="px-3 py-1.5 rounded-lg border border-gray-200 bg-white font-medium text-gray-600 hover:bg-gray-50 disabled:opacity-40 disabled:cursor-not-allowed transition-colors"
                on:click={() => goToPage(currentPage + 1)}
                disabled={currentPage === totalPages}
              >
                Sau
              </button>
            </div>
          </div>
        {/if}

      </div>

    </div>
  {/if}
</section>
