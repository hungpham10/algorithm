<script>
    import { onMount, onDestroy, untrack } from "svelte";
    import LightChart from "$lib/trading/chart.js";
    import LightDraw from "$lib/trading/draw.js";
    import IndicatorList from "./IndicatorList.svelte";
    import IndicatorConfig from "./IndicatorConfig.svelte";
    import {
        INDICATOR_DEFS,
        getDefaultParams,
    } from "$lib/trading/indicators/index.js";

    /** Props */
    let {
        broker = "binance",
        symbol = "BTCUSDT",
        title = "Bitcoin / USDT",
        subtitle = "Binance · Realtime",
    } = $props();

    // ===================== Model =====================
    let chartEngine = $state(
        untrack(
            () =>
                new LightChart({
                    broker,
                    symbol,
                    title,
                    subtitle,
                    theme: "light",
                }),
        ),
    );

    let drawingTools = new LightDraw();
    let activeDrawingTool = $state("cursor");

    let activeResolution = $state("1H");

    // Popup state
    let showIndicatorList = $state(false);
    let configPopup = $state(null); // { name, params, paramDefs, isActive } | null
    let activeIndicatorNames = $state({});

    let chartContainer;
    let tc = $derived(chartEngine.classes);

    function _refreshActiveNames() {
        const names = {};
        for (const name of chartEngine._activeIndicators.keys()) {
            names[name] = true;
        }
        activeIndicatorNames = names;
    }

    // --- Mount / Destroy ---
    onMount(() => {
        if (!chartEngine.mount(chartContainer)) {
            return;
        }

        const dt = drawingTools;
        dt.setChart(
            chartEngine.chart,
            chartContainer,
            chartEngine.candlestickSeries,
        );

        dt.onChange = (state) => {
            activeDrawingTool = state.activeTool;
        };

        // Callback khi indicator cần mở config popup
        chartEngine.onIndicatorConfigRequest = (name, params, isActive) => {
            const def = INDICATOR_DEFS[name];
            configPopup = {
                name,
                params,
                paramDefs: def?.params || [],
                isActive,
            };
        };
    });

    onDestroy(() => {
        chartEngine.destroy();
        drawingTools.destroy();
    });

    // --- Handlers ---
    function selectDrawingTool(toolId) {
        activeDrawingTool = toolId;
        drawingTools?.selectTool(toolId);
    }

    function clearAllDrawings() {
        activeDrawingTool = "cursor";
        drawingTools?.clearAll();
    }

    function handleKeydown(event) {
        drawingTools?.handleKeydown(event);
        if (drawingTools) {
            activeDrawingTool = drawingTools.activeTool;
        }
        if (event.key === "Escape") {
            showIndicatorList = false;
            configPopup = null;
        }
    }

    // Indicator list handlers
    async function toggleIndicatorFromList(name, active) {
        if (active) {
            const params = getDefaultParams(name);
            await chartEngine.applyIndicatorConfig(name, params);
        } else {
            chartEngine.removeIndicator(name);
        }
        _refreshActiveNames();
    }

    function openIndicatorConfig(name) {
        showIndicatorList = false;
        chartEngine.requestIndicatorConfig(name);
    }

    // Config popup handlers
    async function handleIndicatorApply(params) {
        if (!configPopup) return;
        await chartEngine.applyIndicatorConfig(configPopup.name, params);
        _refreshActiveNames();
        configPopup = null;
    }

    function handleIndicatorRemove() {
        if (!configPopup) return;
        chartEngine.removeIndicator(configPopup.name);
        _refreshActiveNames();
        configPopup = null;
    }

    function changeChartType(type) {
        chartEngine.changeChartType(type);
    }

    function selectResolution(res) {
        activeResolution = res.value;
        chartEngine.setResolution(res.value);
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<!-- Indicator List Popup -->
{#if showIndicatorList}
    <IndicatorList
        indicators={chartEngine.indicators}
        activeNames={activeIndicatorNames}
        onToggle={toggleIndicatorFromList}
        onConfigure={openIndicatorConfig}
        onclose={() => (showIndicatorList = false)}
    />
{/if}

<!-- Indicator Config Popup -->
{#if configPopup}
    <IndicatorConfig
        name={configPopup.name}
        params={configPopup.params}
        paramDefs={configPopup.paramDefs}
        isActive={configPopup.isActive}
        onapply={handleIndicatorApply}
        onremove={handleIndicatorRemove}
        onclose={() => (configPopup = null)}
    />
{/if}

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
                  {activeResolution === res.value
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

                    <!-- Nút Chỉ báo -->
                    <button
                        class="flex items-center gap-1 px-3 py-1.5 text-xs font-semibold rounded border transition-all cursor-pointer
                        {Object.keys(activeIndicatorNames).length > 0
                            ? 'bg-indigo-600 text-white border-indigo-600 shadow-sm'
                            : tc.indicatorBtn}"
                        onclick={() => (showIndicatorList = !showIndicatorList)}
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="14"
                            height="14"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2.5"
                            class="shrink-0"
                            ><polyline
                                points="22 12 18 12 15 21 9 3 6 12 2 12"
                            /></svg
                        >
                        Chỉ báo
                        {#if Object.keys(activeIndicatorNames).length > 0}
                            <span
                                class="inline-flex items-center justify-center w-5 h-5 text-[10px] font-bold rounded-full bg-white/20"
                                >{Object.keys(activeIndicatorNames)
                                    .length}</span
                            >
                        {/if}
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="12"
                            height="12"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            class="shrink-0 opacity-60"
                            ><polyline points="6 9 12 15 18 9" /></svg
                        >
                    </button>
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
