<script>
    import { onMount, onDestroy, untrack } from "svelte";
    import Chart from "$lib/trading/chart.js";
    import Draw from "$lib/trading/draw.js";

    /** Props — destructure ngay để tránh Svelte 5 state_referenced_locally */
    let {
        broker = "binance",
        symbol = "BTCUSDT",
        title = "Bitcoin / USDT",
        subtitle = "Binance · Realtime",
    } = $props();

    // ===================== Model =====================
    // untrack để đọc init-only props một lần, tránh state_referenced_locally
    let chartEngine = $state(
        untrack(
            () =>
                new Chart({ broker, symbol, title, subtitle, theme: "light" }),
        ),
    );

    // Drawing tools không thuộc Chart model
    let drawingTools = $state(new Draw());
    let activeDrawingTool = $state("cursor");
    let selectedOverlayId = null;

    // --- DOM ref ---
    let chartContainer;

    // --- Theme classes reactive ---
    let tc = $derived(chartEngine.classes);

    // --- Mount / Destroy ---
    onMount(() => {
        if (!chartEngine.mount(chartContainer)) {
            return;
        }

        drawingTools.setChart(chartEngine.chart);

        // Trạng thái drawing → reactive state
        drawingTools.onChange = (state) => {
            activeDrawingTool = state.activeTool;
            selectedOverlayId = state.selectedOverlayId;
        };
    });

    onDestroy(() => {
        chartEngine.destroy();
    });

    // --- Handlers ---
    function selectDrawingTool(toolId) {
        drawingTools?.selectTool(toolId);
    }

    function clearAllDrawings() {
        drawingTools?.clearAll();
    }

    function handleKeydown(event) {
        drawingTools?.handleKeydown(event);
    }

    function toggleIndicator(name, paneId = "candle_pane") {
        chartEngine.toggleIndicator(name, paneId);
    }

    function changeChartType(type) {
        chartEngine.changeChartType(type);
    }

    function selectResolution(res) {
        chartEngine.setResolution(res.value);
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
    class="w-full max-w-400 mx-auto p-2 md:p-4 rounded-xl shadow-2xl transition-colors {tc.outer}"
>
    <div
        class="flex flex-wrap items-center justify-between pb-3 mb-3 gap-4 px-2 transition-colors {tc.headerBorder}"
    >
        <div class="flex items-center gap-3">
            <span
                class="bg-amber-500/10 text-amber-500 font-bold px-2 py-0.5 rounded text-xs border border-amber-500/20"
                >{broker.toUpperCase()}</span
            >
            <div>
                <h1 class="text-xl font-bold tracking-tight {tc.titleText}">
                    {title}
                </h1>
                <p class="text-xs {tc.subtitleText}">{subtitle}</p>
            </div>
        </div>
    </div>

    <div
        class="flex rounded-xl overflow-hidden relative transition-colors {tc.chartArea}"
    >
        <div
            class="w-14 flex flex-col items-center py-2 gap-1 z-10 shrink-0 shadow-lg transition-colors {tc.sidebar}"
        >
            {#each drawingTools.tools as tool}
                <button
                    class="w-10 h-10 flex flex-col items-center justify-center rounded-lg transition-all text-lg group relative cursor-pointer
            {activeDrawingTool === tool.id
                        ? 'bg-indigo-600 text-white shadow-md'
                        : tc.toolBtnInactive}"
                    onclick={() => selectDrawingTool(tool.id)}
                >
                    <span>{tool.icon}</span>
                    <span
                        class="absolute left-14 text-xs px-2 py-1 rounded opacity-0 pointer-events-none group-hover:opacity-100 transition-opacity whitespace-nowrap z-50 shadow-md {tc.tooltip}"
                    >
                        {tool.label}
                    </span>
                </button>
            {/each}

            <div class="w-8 my-2 transition-colors {tc.divider}"></div>

            <button
                class="w-10 h-10 flex items-center justify-center rounded-lg transition-all cursor-pointer group relative {tc.deleteBtn}"
                onclick={clearAllDrawings}
            >
                🗑️
                <span
                    class="absolute left-14 text-xs px-2 py-1 rounded opacity-0 pointer-events-none group-hover:opacity-100 transition-opacity whitespace-nowrap z-50 shadow-md {tc.tooltip}"
                >
                    Xóa tất cả hình vẽ
                </span>
            </button>
        </div>

        <div
            class="flex-1 flex flex-col min-w-0 transition-colors {tc.chartBg}"
        >
            <div
                class="h-12 flex items-center justify-between px-4 gap-2 overflow-x-auto whitespace-nowrap transition-colors {tc.toolbar}"
            >
                <div class="flex items-center gap-3">
                    <div
                        class="flex items-center p-0.5 rounded-lg border transition-colors {tc.segGroup}"
                    >
                        {#each chartEngine.resolutions as res}
                            <button
                                class="px-3 py-1 text-xs font-semibold rounded-md transition-all cursor-pointer
                  {chartEngine.activeResolution === res.value
                                    ? 'bg-indigo-600 text-white shadow-sm'
                                    : tc.resInactive}"
                                onclick={() => selectResolution(res)}
                                >{res.label}</button
                            >
                        {/each}
                    </div>

                    <div
                        class="h-4 w-px transition-colors {tc.separator}"
                    ></div>

                    <div class="flex items-center gap-1">
                        <button
                            class="px-2 py-1 text-xs font-medium rounded border border-transparent transition-all cursor-pointer {tc.chartTypeBtn}"
                            onclick={() => changeChartType("candle_solid")}
                            >📊 Nến</button
                        >
                        <button
                            class="px-2 py-1 text-xs font-medium rounded border border-transparent transition-all cursor-pointer {tc.chartTypeBtn}"
                            onclick={() => changeChartType("area")}
                            >📈 Vùng</button
                        >
                        <button
                            class="px-2 py-1 text-xs font-medium rounded border border-transparent transition-all cursor-pointer {tc.chartTypeBtn}"
                            onclick={() => changeChartType("ohlc")}
                            >➖ Thanh</button
                        >
                    </div>

                    <div
                        class="h-4 w-px transition-colors {tc.separator}"
                    ></div>

                    <div class="flex items-center gap-1">
                        <span
                            class="text-xs font-medium mr-1 {tc.indicatorLabel}"
                            >Chỉ báo:</span
                        >
                        {#each chartEngine.indicators as ind}
                            <button
                                class="px-2 py-1 text-xs font-semibold rounded border transition-all cursor-pointer {chartEngine.activeIndicators.has(
                                    ind.name,
                                )
                                    ? tc.indicatorActiveBtn
                                    : tc.indicatorBtn}"
                                onclick={() =>
                                    toggleIndicator(ind.name, ind.paneId)}
                                >{ind.label}</button
                            >
                        {/each}
                    </div>
                </div>

                <div class="flex items-center gap-2">
                    <div
                        class="text-[10px] font-mono flex items-center gap-1.5 px-2 py-1 rounded border transition-colors {tc.sseBox}"
                    >
                        <span
                            class="h-1.5 w-1.5 rounded-full bg-green-500 animate-pulse"
                        ></span>
                        {chartEngine.stopRealtime ? "SSE STREAMING" : "POLLING"}
                    </div>
                </div>
            </div>

            <div
                bind:this={chartContainer}
                class="w-full h-[550px] md:h-[650px] select-none transition-colors {tc.chartBg}"
            ></div>
        </div>
    </div>
</div>
