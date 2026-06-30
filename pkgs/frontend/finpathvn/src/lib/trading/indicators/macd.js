/**
 * MACD — Moving Average Convergence Divergence indicator
 */
import { LineSeries } from "lightweight-charts";
import { ema } from "../../utils.js";

export const MACD = {
  name: "MACD",
  label: "MACD",
  category: "pane",
  params: [
    {
      name: "fast",
      label: "Fast Period",
      type: "number",
      default: 12,
      min: 1,
      max: 100,
      step: 1,
    },
    {
      name: "slow",
      label: "Slow Period",
      type: "number",
      default: 26,
      min: 1,
      max: 200,
      step: 1,
    },
    {
      name: "signal",
      label: "Signal Period",
      type: "number",
      default: 9,
      min: 1,
      max: 100,
      step: 1,
    },
    { name: "color", label: "Color", type: "color", default: "#ff5722" },
  ],
  create(chart, candleSeries, params) {
    const data = candleSeries.data();
    if (!data?.length || data.length < Math.max(params.fast, params.slow))
      return null;

    const emaFast = ema(data, params.fast);
    const emaSlow = ema(data, params.slow);

    // MACD line = EMA(fast) - EMA(slow)
    const macdLine = [];
    for (let i = 0; i < emaSlow.length; i++) {
      const fast = emaFast.find((e) => e.time === emaSlow[i].time);
      if (fast)
        macdLine.push({
          time: emaSlow[i].time,
          value: fast.value - emaSlow[i].value,
        });
    }
    if (!macdLine.length) return null;

    const macdSeries = chart.addSeries(LineSeries, {
      color: params.color,
      lineWidth: 2,
      lastValueVisible: false,
      priceLineVisible: false,
      priceScaleId: "macd",
      autoscaleInfoProvider: () => null,
    });
    macdSeries.setData(macdLine);

    // Signal line = EMA of MACD
    const signalValues = ema(macdLine, params.signal);
    const signalSeries = chart.addSeries(LineSeries, {
      color: "#ffeb3b",
      lineWidth: 1,
      lastValueVisible: false,
      priceLineVisible: false,
      priceScaleId: "macd",
      autoscaleInfoProvider: () => null,
    });
    signalSeries.setData(signalValues);

    // Histogram = MACD - Signal
    const histData = [];
    for (let i = 0; i < signalValues.length; i++) {
      const mac = macdLine.find((m) => m.time === signalValues[i].time);
      if (mac)
        histData.push({
          time: signalValues[i].time,
          value: mac.value - signalValues[i].value,
        });
    }

    const histSeries = chart.addSeries(LineSeries, {
      color: "#4caf50",
      lineWidth: 2,
      lastValueVisible: false,
      priceLineVisible: false,
      priceScaleId: "macd",
      autoscaleInfoProvider: () => null,
    });
    histSeries.setData(histData);

    const macdScale = chart.priceScale("macd");
    if (macdScale) {
      macdScale.applyOptions({ scaleMargins: { top: 0.75, bottom: 0 } });
    }

    return { series: [macdSeries, signalSeries, histSeries], scaleId: "macd" };
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
