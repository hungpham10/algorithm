<script>
    import { onMount, onDestroy } from 'svelte';
    import { init, dispose } from 'klinecharts';

    export let symbol = 'PLX';
    export let resolution = '1D';

    let chartContainer;
    let chart;

    onMount(() => {
        // Khởi tạo chart
        chart = init(chartContainer, {
            styles: {
                grid: { show: true, horizontal: { style: 'dash' } },
                candle: { type: 'candle_solid' }
            }
        });

        // Đổ dữ liệu vào
        chart.applyNewData(generateData());

        // Xử lý resize khi cửa sổ trình duyệt thay đổi
        const handleResize = () => chart.resize();
        window.addEventListener('resize', handleResize);

        return () => {
            window.removeEventListener('resize', handleResize);
            dispose(chartContainer); // Dọn dẹp memory
        };
    });

    // Cập nhật khi symbol hoặc resolution thay đổi
    $: if (chart && (symbol || resolution)) {
        console.log(`Fetching data for ${symbol} at ${resolution}`);
        chart.applyNewData(generateData());
    }
</script>

<div class="chart-wrapper">
    <div class="info">
        <h3>{symbol} - {resolution}</h3>
    </div>
    <div bind:this={chartContainer} class="chart-container"></div>
</div>

<style>
    .chart-wrapper {
        width: 100%;
        height: 500px;
        background: #191b21;
        padding: 20px;
        border-radius: 8px;
    }
    .chart-container {
        width: 100%;
        height: 400px;
    }
    .info {
        color: #eee;
        margin-bottom: 10px;
    }
</style>
