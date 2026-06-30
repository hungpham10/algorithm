/**
 * HEATMAP — Heatmap Levels indicator
 * Vẽ các mức tích luỹ dưới dạng đường kẻ ngang trên chart.
 * Có thể chọn vẽ vô hạn (price line) hoặc giới hạn (line series).
 */
import { LineSeries } from "lightweight-charts";
import { API_V1_ENDPOINT } from "../../fetch.js";
import { resolutionToIntervalHours, hsla } from "./helpers.js";

export const HEATMAP = {
  name: "HEATMAP",
  label: "Heatmap Levels",
  category: "overlay",
  params: [
    {
      name: "lookBack",
      label: "Look Back",
      type: "number",
      default: 200,
      min: 50,
      max: 500,
      step: 50,
    },
    {
      name: "overlap",
      label: "Overlap",
      type: "number",
      default: 0,
      min: 0,
      max: 10,
      step: 1,
    },
    {
      name: "numberOfLevels",
      label: "Levels",
      type: "number",
      default: 30,
      min: 10,
      max: 100,
      step: 5,
    },
    {
      name: "extendInfinite",
      label: "Extend Infinite",
      type: "toggle",
      default: true,
      labelOn: "Kéo dài vô hạn",
    },
  ],
  async create(chart, candleSeries, params, context) {
    const now = Math.floor(Date.now() / 1000);
    const resolution = context?.resolution || "1H";
    const broker = context?.broker || "binance";
    const symbol = context?.symbol || "BTCUSDT";
    const intervalInHour = resolutionToIntervalHours(resolution);
    const url = `${API_V1_ENDPOINT}/ohcl/heatmap/${broker}/${symbol}?resolution=${resolution}&now=${now}&look_back=${params.lookBack}&overlap=${params.overlap}&number_of_levels=${params.numberOfLevels}&interval_in_hour=${intervalInHour}`;

    const res = await fetch(url);
    if (!res.ok) throw new Error(`HTTP ${res.status}`);
    const json = await res.json();
    if (!json.heatmap?.ranges) return null;

    const { levels, ranges } = json.heatmap;
    const numRanges = ranges.length;
    const extendInfinite = params.extendInfinite !== false; // default true

    /**
     * Mỗi range gồm [beginIdx, centerIdx, endIdx] là index trong mảng levels.
     * Center: đường chính (solid, dày)
     * Begin/End: đường biên (dashed, mảnh)
     */

    if (extendInfinite) {
      // ── Dùng createPriceLine: đường kéo dài vô hạn ──
      /** @type {import('lightweight-charts').IPriceLine[]} */
      const priceLines = [];

      for (let i = 0; i < numRanges; i++) {
        const [beginIdx, centerIdx, endIdx] = ranges[i];
        const intensity = numRanges > 1 ? 1 - i / (numRanges - 1) : 1;

        const addLine = (price, isCenter) => {
          try {
            const pl = candleSeries.createPriceLine({
              price,
              color: hsla(intensity, isCenter),
              lineWidth: isCenter ? 2 : 1,
              lineStyle: isCenter ? 0 : 2, // 0=Solid, 2=Dashed
              axisLabelVisible: false,
            });
            priceLines.push(pl);
          } catch (_) {}
        };

        addLine(levels[centerIdx], true);
        addLine(levels[beginIdx], false);
        addLine(levels[endIdx], false);
      }

      return { series: priceLines, isPriceLine: true, candleSeries };
    } else {
      // ── Dùng LineSeries: đường chỉ trong vùng dữ liệu ──
      // Lấy khoảng thời gian từ dữ liệu nến để vẽ line
      const candleData = candleSeries.data();
      const firstTime = candleData?.[0]?.time ?? now - 86400 * 30;
      const lastTime = candleData?.[candleData.length - 1]?.time ?? now;

      /** @type {import('lightweight-charts').ISeriesApi[]} */
      const lines = [];

      for (let i = 0; i < numRanges; i++) {
        const [beginIdx, centerIdx, endIdx] = ranges[i];
        const intensity = numRanges > 1 ? 1 - i / (numRanges - 1) : 1;

        const addLine = (price, isCenter) => {
          try {
            const s = chart.addSeries(LineSeries, {
              color: hsla(intensity, isCenter),
              lineWidth: isCenter ? 2 : 1,
              lineStyle: isCenter ? 0 : 2,
              lastValueVisible: false,
              priceLineVisible: false,
              autoscaleInfoProvider: () => null,
            });
            s.setData([
              { time: firstTime, value: price },
              { time: lastTime, value: price },
            ]);
            lines.push(s);
          } catch (_) {}
        };

        addLine(levels[centerIdx], true);
        addLine(levels[beginIdx], false);
        addLine(levels[endIdx], false);
      }

      return { series: lines, isPriceLine: false };
    }
  },
  destroy(handle) {
    if (!handle?.series) return;
    if (handle.isPriceLine && handle.candleSeries) {
      for (const pl of handle.series) {
        try {
          handle.candleSeries.removePriceLine(pl);
        } catch (_) {}
      }
    } else {
      for (const s of handle.series) {
        try {
          s.chart?.removeSeries(s);
        } catch (_) {}
      }
    }
  },
};
