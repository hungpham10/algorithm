<script>
    /**
     * IndicatorList — Popup danh sách indicator
     * - Toggle checkbox: bật/tắt indicator (dùng default params)
     * - Nút răng cưa ⚙: mở config popup để cấu hình trước khi hiện
     */
    let {
        indicators = [],
        activeNames = {},
        onToggle = () => {},
        onConfigure = () => {},
        onclose = () => {},
    } = $props();

    function handleKeydown(e) {
        if (e.key === "Escape") onclose();
    }

    function handleOverlayClick(e) {
        if (e.target === e.currentTarget) onclose();
    }
</script>

<svelte:window onkeydown={handleKeydown} />

<div
    class="fixed inset-0 z-50 flex items-start justify-center pt-24 bg-black/20 backdrop-blur-sm"
    onclick={handleOverlayClick}
>
    <div
        class="bg-white rounded-xl shadow-2xl border border-gray-200 w-72 max-w-[90vw] overflow-hidden animate-dropIn"
        role="dialog"
        aria-label="Danh sách chỉ báo"
    >
        <!-- Header -->
        <div
            class="flex items-center justify-between px-4 py-3 border-b border-gray-200 bg-gray-50"
        >
            <h3 class="font-semibold text-sm text-gray-800">Chỉ báo</h3>
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

        <!-- Danh sách -->
        <div class="py-1 max-h-80 overflow-y-auto">
            {#each indicators as ind}
                {@const isActive = !!activeNames[ind.name]}
                <div
                    class="flex items-center justify-between px-4 py-2.5 hover:bg-gray-50 transition-colors group"
                >
                    <!-- Checkbox toggle + tên -->
                    <label
                        class="flex items-center gap-3 cursor-pointer flex-1 min-w-0"
                    >
                        <input
                            type="checkbox"
                            checked={isActive}
                            onchange={(e) =>
                                onToggle(ind.name, e.target.checked)}
                            class="w-4 h-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500 cursor-pointer shrink-0"
                        />
                        <span
                            class="text-sm font-medium text-gray-700 truncate"
                        >
                            {ind.label}
                        </span>
                    </label>

                    <!-- Nút răng cưa → cấu hình indicator -->
                    <button
                        class="w-7 h-7 flex items-center justify-center rounded-md text-gray-300 hover:text-indigo-600 hover:bg-indigo-50 opacity-0 group-hover:opacity-100 transition-all cursor-pointer shrink-0"
                        onclick={() => onConfigure(ind.name)}
                        aria-label="Cấu hình {ind.label}"
                        title="Cấu hình"
                    >
                        <svg
                            xmlns="http://www.w3.org/2000/svg"
                            width="14"
                            height="14"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="2"
                            ><circle cx="12" cy="12" r="3" /><path
                                d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"
                            /></svg
                        >
                    </button>
                </div>
            {/each}
        </div>

        {#if indicators.length === 0}
            <div class="px-4 py-6 text-center text-sm text-gray-400">
                Không có chỉ báo nào
            </div>
        {/if}
    </div>
</div>

<style>
    @keyframes dropIn {
        from {
            opacity: 0;
            transform: translateY(-8px) scale(0.98);
        }
        to {
            opacity: 1;
            transform: translateY(0) scale(1);
        }
    }
    .animate-dropIn {
        animation: dropIn 0.12s ease-out;
    }
</style>
