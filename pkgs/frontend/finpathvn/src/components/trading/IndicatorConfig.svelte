<script>
    /**
     * IndicatorConfig — Popup cấu hình indicator
     * Dispatch đến component config riêng của từng indicator.
     */
    import DefaultConfig from "./indicators/DefaultConfig.svelte";
    import MaConfig from "./indicators/MaConfig.svelte";
    import EmaConfig from "./indicators/EmaConfig.svelte";
    import BollConfig from "./indicators/BollConfig.svelte";
    import RsiConfig from "./indicators/RsiConfig.svelte";
    import MacdConfig from "./indicators/MacdConfig.svelte";
    import HeatmapConfig from "./indicators/HeatmapConfig.svelte";

    /** Map tên indicator → component config */
    const CONFIG_MAP = {
        MA: MaConfig,
        EMA: EmaConfig,
        BOLL: BollConfig,
        RSI: RsiConfig,
        MACD: MacdConfig,
        HEATMAP: HeatmapConfig,
    };

    let {
        name = "",
        params = {},
        paramDefs = [],
        isActive = false,
        onapply = () => {},
        onremove = () => {},
        onclose = () => {},
    } = $props();

    let localParams = $state({ ...params });

    $effect(() => {
        localParams = { ...params };
    });

    function updateParam(paramName, value) {
        localParams[paramName] = value;
    }

    function handleApply() {
        onapply({ ...localParams });
    }

    function handleOverlayClick(e) {
        if (e.target === e.currentTarget) onclose();
    }

    function handleKeydown(e) {
        if (e.key === "Escape") onclose();
    }

    // Lấy component config phù hợp
    let ConfigComponent = $derived(CONFIG_MAP[name] || DefaultConfig);
</script>

<svelte:window onkeydown={handleKeydown} />

{#key name}
    <div
        class="fixed inset-0 z-50 flex items-center justify-center bg-black/40 backdrop-blur-sm"
        onclick={handleOverlayClick}
    >
        <div
            class="bg-white rounded-xl shadow-2xl border border-gray-200 w-80 max-w-[90vw] overflow-hidden animate-fadeIn"
            role="dialog"
            aria-label="Config {name}"
        >
            <!-- Header -->
            <div
                class="flex items-center justify-between px-4 py-3 border-b border-gray-200 bg-gray-50"
            >
                <h3 class="font-semibold text-sm text-gray-800">
                    {name}
                    <span class="text-gray-400 font-normal ml-1"
                        >Configuration</span
                    >
                </h3>
                <button
                    class="w-7 h-7 flex items-center justify-center rounded-md text-gray-400 hover:text-gray-600 hover:bg-gray-200 transition-all cursor-pointer"
                    onclick={onclose}
                    aria-label="Close"
                >
                    <svg
                        xmlns="http://www.w3.org/2000/svg"
                        width="16"
                        height="16"
                        viewBox="0 0 24 24"
                        fill="none"
                        stroke="currentColor"
                        stroke-width="2"
                        ><line x1="18" y1="6" x2="6" y2="18" /><line
                            x1="6"
                            y1="6"
                            x2="18"
                            y2="18"
                        /></svg
                    >
                </button>
            </div>

            <!-- Body -->
            <div class="px-4 py-3 max-h-64 overflow-y-auto">
                <ConfigComponent
                    params={localParams}
                    {paramDefs}
                    onupdate={updateParam}
                />
            </div>

            <!-- Footer -->
            <div
                class="flex items-center justify-between px-4 py-3 border-t border-gray-200 bg-gray-50"
            >
                {#if isActive}
                    <button
                        class="px-3 py-1.5 text-xs font-medium text-red-600 hover:text-red-700 hover:bg-red-50 rounded-md transition-all cursor-pointer"
                        onclick={onremove}
                    >
                        Remove
                    </button>
                {:else}
                    <span></span>
                {/if}
                <div class="flex items-center gap-2">
                    <button
                        class="px-3 py-1.5 text-xs font-medium text-gray-600 hover:text-gray-800 hover:bg-gray-200 rounded-md transition-all cursor-pointer"
                        onclick={onclose}
                    >
                        Cancel
                    </button>
                    <button
                        class="px-4 py-1.5 text-xs font-semibold text-white bg-indigo-600 hover:bg-indigo-700 rounded-md transition-all cursor-pointer shadow-sm"
                        onclick={handleApply}
                    >
                        Apply
                    </button>
                </div>
            </div>
        </div>
    </div>
{/key}

<style>
    @keyframes fadeIn {
        from {
            opacity: 0;
            transform: scale(0.95);
        }
        to {
            opacity: 1;
            transform: scale(1);
        }
    }
    .animate-fadeIn {
        animation: fadeIn 0.15s ease-out;
    }
</style>
