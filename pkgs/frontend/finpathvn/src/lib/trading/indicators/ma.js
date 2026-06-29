/**
 * MA — Moving Average indicator
 */
import { LineSeries } from "lightweight-charts";
import { sma } from "../../utils.js";

export const MA = {
  name: "MA",
  label: "MA",
  category: "overlay",
  params: [
    {
      name: "period",
      label: "Period",
      type: "number",
      default: 20,
      min: 1,
      max: 200,
      step: 1,
    },
    { name: "color", label: "Color", type: "color", default: "#e91e63" },
    {
      name: "lineWidth",
      label: "Line Width",
      type: "number",
      default: 2,
      min: 1,
      max: 5,
      step: 1,
    },
  ],
  create(chart, candleSeries, params) {
    const data = candleSeries.data();
    if (!data?.length) return null;
    const values = sma(data, params.period);
    if (!values.length) return null;
    const series = chart.addSeries(LineSeries, {
      color: params.color,
      lineWidth: params.lineWidth,
      lastValueVisible: false,
      priceLineVisible: false,
      autoscaleInfoProvider: () => null,
    });
    series.setData(values);
    return { series: [series] };
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
