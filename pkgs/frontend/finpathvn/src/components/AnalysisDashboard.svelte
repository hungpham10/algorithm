<script>
  import TradingChart from './TradingChart.svelte';

  // Trạng thái (State) quản lý toàn bộ trang
  let symbol = 'PLX';
  let resolution = '1H';
  let broker = 'dnse';

  const symbols = ['PLX', 'FPT', 'VCB', 'MSN', 'HPG'];
  const resolutions = [
    { label: '1 Giờ', value: '1H' },
    { label: '1 Ngày', value: '1D' }
  ];
</script>

<main class="analysis-container">
    <!-- HEADER: Điều khiển -->
    <div class="controls-bar">
        <!-- Ràng buộc biến symbol trực tiếp -->
        <select bind:value={symbol} class="styled-select">
            {#each symbols as s}
                <option value={s}>{s}</option>
            {/each}
        </select>

        <div class="btn-group">
            {#each resolutions as r}
                <button
                    class="res-btn"
                    class:active={resolution === r.value}
                    on:click={() => resolution = r.value}
                >
                    {r.label}
                </button>
            {/each}
        </div>

        <div class="status">
            Live Data: <span class="highlight">{symbol}</span>
            | Khung: <span class="highlight">{resolution}</span>
        </div>
    </div>

    <div class="chart-view">
        <TradingChart {broker} {symbol} {resolution} />
    </div>
</main>

<style>
    .analysis-container { display: flex; flex-direction: column; height: 100vh; background: #131722; color: #d1d4dc; }
    .controls-bar {
        height: 50px;
        background: #1e222d;
        border-bottom: 1px solid #2a2e39;
        display: flex;
        align-items: center;
        padding: 0 15px;
        gap: 20px;
    }
    .chart-view { flex: 1; position: relative; }

    .styled-select {
        background: #2a2e39;
        color: white;
        border: 1px solid #363c4e;
        padding: 5px 10px;
        border-radius: 4px;
        outline: none;
    }

    .res-btn {
        background: #2a2e39;
        color: #b2b5be;
        border: none;
        padding: 5px 12px;
        cursor: pointer;
        border-radius: 4px;
        font-size: 12px;
        margin-right: 5px;
        transition: all 0.2s;
    }
    .res-btn.active { background: #2962ff; color: white; }
    .res-btn:hover { background: #363c4e; }

    .status { color: #848e9c; font-size: 12px; margin-left: auto; }
    .highlight { color: #2962ff; font-weight: bold; }
</style>
