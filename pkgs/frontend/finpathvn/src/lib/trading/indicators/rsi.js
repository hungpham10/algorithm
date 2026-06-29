/**
 * RSI — Relative Strength Index indicator
 */
import { LineSeries } from "lightweight-charts";

export const RSI = {
  name: "RSI",
  label: "RSI",
  category: "pane",
  params: [
    { name: "period", label: "Period", type: "number", default: 14, min: 1, max: 100, step: 1 },
    { name: "overbought", label: "Overbought", type: "number", default: 70, min: 50, max: 100, step: 1 },
    { name: "oversold", label: "Oversold", type: "number", default: 30, min: 0, max: 50, step: 1 },
    { name: "color", label: "Color", type: "color", default: "#2196f3" },
  ],
  create(chart, candleSeries, params) {
    const data = candleSeries.data();
    if (!data?.length || data.length < params.period + 1) return null;

    const closes = data.map((d) => d.close);
    const gains = [];
    const losses = [];
    for (let i = 1; i < closes.length; i++) {
      const diff = closes[i] - closes[i - 1];
      gains.push(diff > 0 ? diff : 0);
      losses.push(diff < 0 ? -diff : 0);
    }

    const rsiValues = [];
    let avgGain = gains.slice(0, params.period).reduce((s, v) => s + v, 0) / params.period;
    let avgLoss = losses.slice(0, params.period).reduce((s, v) => s + v, 0) / params.period;

    for (let i = params.period; i < gains.length; i++) {
      avgGain = (avgGain * (params.period - 1) + gains[i]) / params.period;
      avgLoss = (avgLoss * (params.period - 1) + losses[i]) / params.period;
      const rs = avgLoss === 0 ? 100 : avgGain / avgLoss;
      rsiValues.push({ time: data[i + 1].time, value: 100 - 100 / (1 + rs) });
    }

    const rsiSeries = chart.addSeries(LineSeries, {
      color: params.color, lineWidth: 2,
      lastValueVisible: false, priceLineVisible: false,
      priceScaleId: "rsi",
      autoscaleInfoProvider: () => null,
    });
    rsiSeries.setData(rsiValues);

    const rsiScale = chart.priceScale("rsi");
    if (rsiScale) {
      rsiScale.applyOptions({ scaleMargins: { top: 0.8, bottom: 0 } });
    }

    // Overbought line
    const obLine = chart.addSeries(LineSeries, {
      color: "#ef5350", lineWidth: 1, lineStyle: 2,
      lastValueVisible: false, priceLineVisible: false,
      priceScaleId: "rsi",
      autoscaleInfoProvider: () => null,
    });
    obLine.setData([
      { time: data[0].time, value: params.overbought },
      { time: data[data.length - 1].time, value: params.overbought },
    ]);

    // Oversold line
    const osLine = chart.addSeries(LineSeries, {
      color: "#66bb6a", lineWidth: 1, lineStyle: 2,
      lastValueVisible: false, priceLineVisible: false,
      priceScaleId: "rsi",
      autoscaleInfoProvider: () => null,
    });
    osLine.setData([
      { time: data[0].time, value: params.oversold },
      { time: data[data.length - 1].time, value: params.oversold },
    ]);

    return { series: [rsiSeries, obLine, osLine], scaleId: "rsi" };
  },
  destroy(handle) {
    if (!handle?.series) return;
    for (const s of handle.series) {
      try { s.chart?.removeSeries(s); } catch (_) {}
    }
  },
};
