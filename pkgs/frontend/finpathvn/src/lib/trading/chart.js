/**
 * LightChart — Lightweight Charts v5 wrapper
 * Minimal, clean, just works.
 */
import {
  createChart,
  ColorType,
  CandlestickSeries,
  HistogramSeries,
} from "lightweight-charts";
import { getHistoryCandles } from "../api.js";
import { API_V1_ENDPOINT } from "../fetch.js";
import {
  INDICATOR_DEFS,
  getIndicatorList,
  getDefaultParams,
} from "./indicators/index.js";

// ── Tailwind theme classes (used by TradingChart.svelte) ──
const TAILWIND_THEMES = {
  light: {
    outer: "bg-white text-gray-900 border border-gray-200",
    headerBorder: "border-b border-gray-200",
    titleText: "text-gray-900",
    subtitleText: "text-gray-500",
    chartArea: "border border-gray-200 bg-white",
    sidebar: "border-r border-gray-200 bg-gray-50",
    toolBtnInactive: "text-gray-500 hover:bg-gray-200",
    tooltip: "bg-gray-900 text-white",
    divider: "border-t border-gray-200",
    deleteBtn: "text-gray-400 hover:text-red-500 hover:bg-red-50",
    chartBg: "bg-white",
    toolbar: "border-b border-gray-200 bg-gray-50/80",
    segGroup: "border-gray-200 bg-gray-100/50",
    resInactive: "text-gray-600 hover:bg-gray-200",
    separator: "bg-gray-200",
    chartTypeBtn: "text-gray-600 hover:bg-gray-200",
    indicatorLabel: "text-gray-500",
    indicatorActiveBtn: "bg-indigo-600 text-white border-indigo-600",
    indicatorBtn: "text-gray-600 border-gray-300 hover:bg-gray-100",
    sseBox: "bg-green-50 border-green-200 text-green-700",
  },
  dark: {
    outer: "bg-slate-900 text-slate-100",
    headerBorder: "border-b border-slate-800",
    titleText: "text-white",
    subtitleText: "text-slate-400",
    chartArea: "border border-slate-800 bg-slate-950",
    sidebar: "border-r border-slate-800 bg-slate-900/90",
    toolBtnInactive: "text-slate-400 hover:bg-slate-800",
    tooltip: "bg-slate-700 text-white",
    divider: "border-t border-slate-800",
    deleteBtn: "text-slate-400 hover:text-red-400 hover:bg-red-900/20",
    chartBg: "bg-slate-950",
    toolbar: "border-b border-slate-800 bg-slate-900/40",
    segGroup: "border-slate-700 bg-slate-800/50",
    resInactive: "text-slate-400 hover:bg-slate-800",
    separator: "bg-slate-800",
    chartTypeBtn: "text-slate-400 hover:bg-slate-800",
    indicatorLabel: "text-slate-400",
    indicatorActiveBtn: "bg-indigo-600 text-white border-indigo-600",
    indicatorBtn: "text-slate-400 border-slate-700 hover:bg-slate-800",
    sseBox: "bg-green-900/20 border-green-800 text-green-400",
  },
};

// ── Canvas themes ──
const CANVAS_THEMES = {
  light: { background: { color: "#ffffff" }, textColor: "#6b7280" },
  dark: { background: { color: "#0f172a" }, textColor: "#cbd5e1" },
};

// Resolution definitions
const RESOLUTIONS = [
  { value: "1m", label: "1m", periodSec: 60 },
  { value: "5m", label: "5m", periodSec: 300 },
  { value: "1H", label: "1H", periodSec: 3600 },
  { value: "4H", label: "4H", periodSec: 14400 },
  { value: "1D", label: "1D", periodSec: 86400 },
];

export default class LightChart {
  constructor(options = {}) {
    this.broker = options.broker || "binance";
    this.symbol = options.symbol || "BTCUSDT";
    this.title = options.title || "";
    this.subtitle = options.subtitle || "";
    this.theme = options.theme || "dark";

    this.chart = null;
    this.candlestickSeries = null;
    this.volumeSeries = null;
    this.currentPrice = 0;
    this.priceChange = 0;
    this.priceChangePercent = 0;
    this.high24h = 0;
    this.low24h = 0;
    this.volume24h = 0;
    this.stopRealtime = null;

    this.activeResolution = "1H";
    this._resolutions = RESOLUTIONS;
    this._indicatorDefs = INDICATOR_DEFS;
    this._indicators = getIndicatorList();
    this._activeIndicators = new Map(); // name → { params, series[] }
    this._isLoading = false;

    /** Callback khi user click indicator — component dùng để hiện popup config */
    this.onIndicatorConfigRequest = null; // (name, params, isActive) => void
  }

  // ── Public accessors ──
  get resolutions() {
    return this._resolutions;
  }
  get indicators() {
    return this._indicators;
  }
  get activeIndicators() {
    return this._activeIndicators;
  }
  get classes() {
    return TAILWIND_THEMES[this.theme] || TAILWIND_THEMES.dark;
  }

  // ── Mount / Destroy ──
  mount(container) {
    if (!container) return false;
    try {
      this.chart = createChart(container, {
        layout: {
          background: {
            type: ColorType.Solid,
            color: CANVAS_THEMES[this.theme].background.color,
          },
          textColor: CANVAS_THEMES[this.theme].textColor,
        },
        timeScale: { timeVisible: true, secondsVisible: false },
      });

      this.candlestickSeries = this.chart.addSeries(CandlestickSeries, {
        upColor: "#16a34a",
        downColor: "#dc2626",
        borderDownColor: "#dc2626",
        borderUpColor: "#16a34a",
        wickDownColor: "#dc2626",
        wickUpColor: "#16a34a",
      });

      // Candles use top 85% of chart height
      this.candlestickSeries.priceScale().applyOptions({
        scaleMargins: { top: 0, bottom: 0.15 },
      });

      this.volumeSeries = this.chart.addSeries(HistogramSeries, {
        color: "#26a69a",
        priceFormat: { type: "volume" },
        priceScaleId: "volume",
      });

      // Volume occupies bottom 15% only
      const volScale = this.chart.priceScale("volume");
      if (volScale) {
        volScale.applyOptions({
          scaleMargins: { top: 0.85, bottom: 0 },
        });
      }

      this._loadData();
      this._subscribeRealtime();
      this._setupScrollLoad();

      return true;
    } catch (err) {
      console.error("[LightChart] Mount error:", err);
      return false;
    }
  }

  destroy() {
    this._removeAllIndicators();
    this.unsubscribeRealtime();
    if (this.chart) {
      this.chart.remove();
      this.chart = null;
    }
  }

  // ── Indicator management ──

  /** Gọi khi user click vào nút indicator — component sẽ hiện popup */
  requestIndicatorConfig(name) {
    const existing = this._activeIndicators.get(name);
    const params = existing ? { ...existing.params } : getDefaultParams(name);
    const isActive = !!existing;
    if (this.onIndicatorConfigRequest) {
      this.onIndicatorConfigRequest(name, params, isActive);
    }
  }

  /** Apply config từ popup — tạo mới hoặc cập nhật indicator */
  async applyIndicatorConfig(name, params) {
    // Xoá cũ nếu có
    this.removeIndicator(name);

    const def = this._indicatorDefs[name];
    if (!def) return;

    const context = {
      broker: this.broker,
      symbol: this.symbol,
      resolution: this.activeResolution,
    };

    try {
      const handle = await def.create(
        this.chart,
        this.candlestickSeries,
        params,
        context,
      );
      if (handle) {
        handle.params = { ...params };
        this._activeIndicators.set(name, handle);
        console.log(`[LightChart] Indicator added: ${name}`, params);
      }
    } catch (err) {
      console.error(`[LightChart] Indicator ${name} error:`, err);
    }
  }

  /** Xoá indicator khỏi chart */
  removeIndicator(name) {
    const handle = this._activeIndicators.get(name);
    if (!handle) return;
    const def = this._indicatorDefs[name];
    if (def?.destroy) {
      def.destroy(handle);
    }
    this._activeIndicators.delete(name);
    console.log(`[LightChart] Indicator removed: ${name}`);
  }

  _removeAllIndicators() {
    for (const name of this._activeIndicators.keys()) {
      this.removeIndicator(name);
    }
  }

  /** Kiểm tra indicator có đang active không */
  isIndicatorActive(name) {
    return this._activeIndicators.has(name);
  }

  // ── Data loading ──
  async _loadData() {
    if (this._isLoading) return;
    this._isLoading = true;
    try {
      const now = Math.floor(Date.now() / 1000);
      const res =
        this._resolutions.find((r) => r.value === this.activeResolution) ||
        this._resolutions[2];
      const limit = 500;
      const from = now - limit * res.periodSec;

      const data = await getHistoryCandles(
        this.broker,
        this.symbol,
        this.activeResolution,
        from,
        now,
        limit,
      );

      if (data?.length) {
        this._setChartData(data);
      }
    } catch (err) {
      console.error("[LightChart] Load data failed:", err);
    } finally {
      this._isLoading = false;
    }
  }

  async _loadMore() {
    if (this._isLoading) return;
    this._isLoading = true;

    try {
      const currentCandles = this.candlestickSeries?.data();
      if (!currentCandles?.length) return;

      const firstTime = currentCandles[0].time;
      const res =
        this._resolutions.find((r) => r.value === this.activeResolution) ||
        this._resolutions[2];
      const limit = 500;
      const timeTo = firstTime - res.periodSec;
      const timeFrom = Math.max(0, timeTo - limit * res.periodSec);

      const raw = await getHistoryCandles(
        this.broker,
        this.symbol,
        this.activeResolution,
        timeFrom,
        timeTo,
        limit,
      );

      if (!raw?.length) return;

      const seen = new Set();
      const deduped = [...raw]
        .sort((a, b) => a.timestamp - b.timestamp)
        .filter((d) => {
          const t = Math.floor(d.timestamp / 1000);
          if (seen.has(t)) return false;
          seen.add(t);
          return true;
        });

      if (!deduped.length) return;

      const existingTimes = new Set(currentCandles.map((c) => c.time));

      const newCandles = deduped
        .filter((d) => !existingTimes.has(Math.floor(d.timestamp / 1000)))
        .map((d) => ({
          time: Math.floor(d.timestamp / 1000),
          open: d.open,
          high: d.high,
          low: d.low,
          close: d.close,
        }));

      const newVolumes = deduped
        .filter((d) => !existingTimes.has(Math.floor(d.timestamp / 1000)))
        .map((d) => ({
          time: Math.floor(d.timestamp / 1000),
          value: d.volume || 0,
          color: d.close >= d.open ? "#26a69a80" : "#f2315580",
        }));

      if (newCandles.length) {
        const mergedCandles = [...newCandles, ...currentCandles];
        const currentVolumes = this.volumeSeries?.data() || [];
        const mergedVolumes = [...newVolumes, ...currentVolumes];
        this.candlestickSeries.setData(mergedCandles);
        this.volumeSeries.setData(mergedVolumes);
      }
    } catch (err) {
      console.error("[LightChart] Load more failed:", err);
    } finally {
      this._isLoading = false;
    }
  }

  _setChartData(data) {
    const seen = new Set();
    const deduped = [...data]
      .sort((a, b) => a.timestamp - b.timestamp)
      .filter((d) => {
        const t = Math.floor(d.timestamp / 1000);
        if (seen.has(t)) return false;
        seen.add(t);
        return true;
      });

    if (!deduped.length) return;

    const candles = deduped.map((d) => ({
      time: Math.floor(d.timestamp / 1000),
      open: d.open,
      high: d.high,
      low: d.low,
      close: d.close,
    }));

    const volumes = deduped.map((d) => ({
      time: Math.floor(d.timestamp / 1000),
      value: d.volume || 0,
      color: d.close >= d.open ? "#26a69a80" : "#f2315580",
    }));

    this.candlestickSeries.setData(candles);
    this.volumeSeries.setData(volumes);
    this._updateStats(deduped);
    this.chart.timeScale().fitContent();
  }

  // ── Scroll to load more ──
  _setupScrollLoad() {
    if (!this.chart) return;
    this.chart
      .timeScale()
      .subscribeVisibleLogicalRangeChange((logicalRange) => {
        if (!logicalRange || this._isLoading) return;
        const barsInfo =
          this.candlestickSeries.barsInLogicalRange(logicalRange);
        if (barsInfo && barsInfo.barsBefore < 50) {
          this._loadMore();
        }
      });
  }

  // ── Realtime SSE ──
  _subscribeRealtime() {
    const url = `${API_V1_ENDPOINT}/ohcl/last-price/${this.broker}/${this.symbol}`;
    const es = new EventSource(url);

    es.addEventListener("tick", (event) => {
      try {
        const p = JSON.parse(event.data);
        if (!p?.tick) return;
        const { price, quantity, timestamp } = p.tick;

        this.currentPrice = price;

        const res =
          this._resolutions.find((r) => r.value === this.activeResolution) ||
          this._resolutions[2];
        const barTime =
          Math.floor(timestamp / 1000 / res.periodSec) * res.periodSec;

        const lastData = this.candlestickSeries.data();
        const last = lastData?.[lastData.length - 1];

        if (last && last.time === barTime) {
          this.candlestickSeries.update({
            time: barTime,
            open: last.open,
            high: Math.max(last.high, price),
            low: Math.min(last.low, price),
            close: price,
          });
        } else {
          this.candlestickSeries.update({
            time: barTime,
            open: price,
            high: price,
            low: price,
            close: price,
          });
        }

        this.volumeSeries.update({
          time: barTime,
          value: quantity || 0,
          color: price >= (last?.close ?? price) ? "#26a69a80" : "#f2315580",
        });
      } catch (e) {
        // skip bad ticks
      }
    });

    this.stopRealtime = () => {
      es.close();
      this.stopRealtime = null;
    };
  }

  unsubscribeRealtime() {
    if (this.stopRealtime) this.stopRealtime();
  }

  // ── Public methods used by template ──
  setTheme(theme) {
    if (theme !== "light" && theme !== "dark") return;
    this.theme = theme;
    if (this.chart) {
      this.chart.applyOptions({
        layout: {
          background: {
            type: ColorType.Solid,
            color: CANVAS_THEMES[theme].background.color,
          },
          textColor: CANVAS_THEMES[theme].textColor,
        },
      });
    }
  }

  setResolution(res) {
    if (res === this.activeResolution) return;
    this.activeResolution = res;
    if (this.chart) {
      this.candlestickSeries.setData([]);
      this.volumeSeries.setData([]);
      this._loadData();
    }
  }

  changeChartType(_type) {
    console.log("[LightChart] Chart type:", _type);
  }

  // ── Internal ──
  _updateStats(data) {
    if (!data?.length) return;
    const latest = data[data.length - 1];
    const prev = data.length > 1 ? data[data.length - 2] : null;

    let maxH = -Infinity,
      minL = Infinity,
      volSum = 0;
    for (const d of data) {
      if (d.high > maxH) maxH = d.high;
      if (d.low < minL) minL = d.low;
      volSum += d.volume || 0;
    }

    this.currentPrice = latest.close;
    this.priceChange = prev ? latest.close - prev.close : 0;
    this.priceChangePercent =
      prev && prev.close ? ((latest.close - prev.close) / prev.close) * 100 : 0;
    this.high24h = maxH > -Infinity ? maxH : 0;
    this.low24h = minL < Infinity ? minL : 0;
    this.volume24h = volSum;
  }
}
