/**
 * BOLL — Bollinger Bands indicator
 */
import { LineSeries } from "lightweight-charts";
import { sma } from "../../utils.js";

export const BOLL = {
  name: "BOLL",
  label: "Bollinger Bands",
  category: "overlay",
  params: [
    {
      name: "period",
      label: "Period",
      type: "number",
      default: 20,
      min: 1,
      max: 100,
      step: 1,
    },
    {
      name: "stdDev",
      label: "Std Dev",
      type: "number",
      default: 2,
      min: 0.5,
      max: 5,
      step: 0.5,
    },
    { name: "color", label: "Color", type: "color", default: "#9c27b0" },
  ],
  create(chart, candleSeries, params) {
    const data = candleSeries.data();
    if (!data?.length || data.length < params.period) return null;

    const mid = sma(data, params.period);
    const upper = [];
    const lower = [];

    for (let i = params.period - 1; i < data.length; i++) {
      const slice = data.slice(i - params.period + 1, i + 1);
      const avg = mid[i - (params.period - 1)].value;
      const variance =
        slice.reduce((s, d) => s + (d.close - avg) ** 2, 0) / params.period;
      const std = Math.sqrt(variance);
      upper.push({ time: data[i].time, value: avg + params.stdDev * std });
      lower.push({ time: data[i].time, value: avg - params.stdDev * std });
    }

    const midSeries = chart.addSeries(LineSeries, {
      color: params.color,
      lineWidth: 1,
      lastValueVisible: false,
      priceLineVisible: false,
      autoscaleInfoProvider: () => null,
    });
    midSeries.setData(mid);

    const upperSeries = chart.addSeries(LineSeries, {
      color: params.color,
      lineWidth: 1,
      lineStyle: 2,
      lastValueVisible: false,
      priceLineVisible: false,
      autoscaleInfoProvider: () => null,
    });
    upperSeries.setData(upper);

    const lowerSeries = chart.addSeries(LineSeries, {
      color: params.color,
      lineWidth: 1,
      lineStyle: 2,
      lastValueVisible: false,
      priceLineVisible: false,
      autoscaleInfoProvider: () => null,
    });
    lowerSeries.setData(lower);

    return { series: [midSeries, upperSeries, lowerSeries] };
  },
  destroy(handle) {
    if (!handle?.series) return;
    for (const s of handle.series) {
      try {
        s.chart?.removeSeries(s);
      } catch (_) {}
    }
  },
};
