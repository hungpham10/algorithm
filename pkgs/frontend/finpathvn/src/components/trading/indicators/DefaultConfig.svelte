<script>
    /**
     * DefaultConfig — Giao diện config generic cho indicator đơn giản
     * Props: params, paramDefs, onUpdate
     */
    let { params = {}, paramDefs = [], onUpdate = () => {} } = $props();
</script>

<div class="space-y-3">
    {#each paramDefs as def}
        <div class="flex flex-col gap-1">
            <label
                class="text-xs font-medium text-gray-600"
                for="param-{def.name}"
            >
                {def.label}
                {#if def.type === "number" && def.min !== undefined}
                    <span class="text-gray-400 font-normal">
                        ({def.min} – {def.max})
                    </span>
                {/if}
            </label>

            {#if def.type === "color"}
                <div class="flex items-center gap-2">
                    <input
                        id="param-{def.name}"
                        type="color"
                        value={params[def.name] ?? def.default}
                        oninput={(e) => onUpdate(def.name, e.target.value)}
                        class="w-10 h-8 p-0.5 rounded border border-gray-300 cursor-pointer"
                    />
                    <span class="text-xs font-mono text-gray-500">
                        {params[def.name] ?? def.default}
                    </span>
                </div>
            {:else if def.type === "toggle"}
                <label class="flex items-center gap-2 cursor-pointer">
                    <input
                        type="checkbox"
                        checked={params[def.name] ?? def.default}
                        onchange={(e) => onUpdate(def.name, e.target.checked)}
                        class="w-4 h-4 rounded border-gray-300 text-indigo-600 focus:ring-indigo-500 cursor-pointer"
                    />
                    <span class="text-xs text-gray-500">{def.labelOn ?? "Enabled"}</span>
                </label>
            {:else}
                <input
                    id="param-{def.name}"
                    type={def.type === "number" ? "range" : "text"}
                    min={def.min}
                    max={def.max}
                    step={def.step}
                    value={params[def.name] ?? def.default}
                    oninput={(e) =>
                        onUpdate(
                            def.name,
                            def.type === "number"
                                ? Number(e.target.value)
                                : e.target.value,
                        )}
                    class={def.type === "number"
                        ? "w-full h-2 accent-indigo-600 cursor-pointer"
                        : "w-full px-2 py-1.5 text-xs border border-gray-300 rounded-md"}
                />
                {#if def.type === "number"}
                    <span class="text-xs font-mono text-gray-500 text-right">
                        {params[def.name] ?? def.default}
                    </span>
                {/if}
            {/if}
        </div>
    {/each}
</div>
