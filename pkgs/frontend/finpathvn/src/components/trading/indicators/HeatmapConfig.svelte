<script>
    /**
     * HeatmapConfig — Giao diện config riêng cho Heatmap Levels
     */
    let { params = {}, paramDefs = [], onUpdate = () => {} } = $props();

    // Helper để lấy def theo tên
    function getDef(name) {
        return paramDefs.find((d) => d.name === name);
    }
</script>

<div class="space-y-3">
    <!-- Look Back -->
    <div class="flex flex-col gap-1">
        <label class="text-xs font-medium text-gray-600" for="param-lookBack">
            Look Back
            <span class="text-gray-400 font-normal">(50 – 500)</span>
        </label>
        <input
            id="param-lookBack"
            type="range"
            min={getDef("lookBack")?.min ?? 50}
            max={getDef("lookBack")?.max ?? 500}
            step={getDef("lookBack")?.step ?? 50}
            value={params.lookBack ?? 200}
            oninput={(e) => onUpdate("lookBack", Number(e.target.value))}
            class="w-full h-2 accent-indigo-600 cursor-pointer"
        />
        <span class="text-xs font-mono text-gray-500 text-right">{params.lookBack ?? 200}</span>
    </div>

    <!-- Overlap -->
    <div class="flex flex-col gap-1">
        <label class="text-xs font-medium text-gray-600" for="param-overlap">
            Overlap
            <span class="text-gray-400 font-normal">(0 – 10)</span>
        </label>
        <input
            id="param-overlap"
            type="range"
            min={getDef("overlap")?.min ?? 0}
            max={getDef("overlap")?.max ?? 10}
            step={getDef("overlap")?.step ?? 1}
            value={params.overlap ?? 0}
            oninput={(e) => onUpdate("overlap", Number(e.target.value))}
            class="w-full h-2 accent-indigo-600 cursor-pointer"
        />
        <span class="text-xs font-mono text-gray-500 text-right">{params.overlap ?? 0}</span>
    </div>

    <!-- Levels -->
    <div class="flex flex-col gap-1">
        <label class="text-xs font-medium text-gray-600" for="param-levels">
            Levels
            <span class="text-gray-400 font-normal">(10 – 100)</span>
        </label>
        <input
            id="param-levels"
            type="range"
            min={getDef("numberOfLevels")?.min ?? 10}
            max={getDef("numberOfLevels")?.max ?? 100}
            step={getDef("numberOfLevels")?.step ?? 5}
            value={params.numberOfLevels ?? 30}
            oninput={(e) => onUpdate("numberOfLevels", Number(e.target.value))}
            class="w-full h-2 accent-indigo-600 cursor-pointer"
        />
        <span class="text-xs font-mono text-gray-500 text-right">{params.numberOfLevels ?? 30}</span>
    </div>

    <!-- Divider -->
    <div class="border-t border-gray-200 pt-2"></div>

    <!-- Extend Infinite toggle -->
    <div class="flex flex-col gap-1">
        <label class="text-xs font-medium text-gray-600">Hiển thị</label>
        <label class="flex items-center gap-3 cursor-pointer py-1">
            <input
                type="checkbox"
                checked={params.extendInfinite ?? true}
                onchange={(e) => onUpdate("extendInfinite", e.target.checked)}
                class="w-4 h-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
            />
            <div class="flex flex-col">
                <span class="text-sm font-medium text-gray-700">Kéo dài vô hạn</span>
                <span class="text-xs text-gray-400">
                    {params.extendInfinite ?? true
                        ? "Đường kẻ ngang trải dài toàn bộ chart"
                        : "Đường kẻ chỉ hiển thị trong vùng dữ liệu"}
                </span>
            </div>
        </label>
    </div>
</div>
