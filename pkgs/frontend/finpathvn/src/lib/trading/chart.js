/**
 * Chart — Quản lý klinecharts core: init, data loader, SSE, indicators & themes
 */
import { init, dispose } from "klinecharts";
import { getHistoryCandles } from "../api.js";
import { API_V1_ENDPOINT } from "../fetch.js";

// ===================== TAILWIND THEMES =====================

const TAILWIND_THEMES = {
  light: {
    outer: "bg-white text-gray-900 border border-gray-200",
    headerBorder: "border-b border-gray-200",
    titleText: "text-gray-900",
    subtitleText: "text-gray-500",
    labelMuted: "text-gray-500",
    valueMuted: "text-gray-600",
    chartArea: "border border-gray-200 bg-white",
    sidebar: "border-r border-gray-200 bg-gray-50",
    toolBtnInactive: "text-gray-500 hover:text-gray-800 hover:bg-gray-200",
    tooltip: "bg-white border border-gray-200 text-gray-700",
    divider: "border-t border-gray-200",
    deleteBtn: "text-gray-500 hover:text-red-500 hover:bg-red-50",
    chartBg: "bg-white",
    toolbar: "border-b border-gray-200 bg-gray-50/80",
    segGroup: "bg-gray-100 border-gray-200",
    resInactive: "text-gray-500 hover:text-gray-800",
    separator: "bg-gray-200",
    chartTypeBtn:
      "text-gray-500 hover:text-gray-800 hover:bg-gray-100 hover:border-gray-200",
    indicatorLabel: "text-gray-500",
    indicatorBtn: "text-gray-600 bg-gray-100 hover:bg-gray-200 border-gray-200",
    indicatorActiveBtn: "bg-indigo-600 text-white shadow-sm border-indigo-600",
    themeBtn: "bg-amber-100 text-amber-700 hover:bg-amber-200 border-amber-200",
    sseBox: "text-gray-500 bg-gray-100 border-gray-200",
  },
  dark: {
    outer: "bg-slate-900 text-slate-100",
    headerBorder: "border-b border-slate-800",
    titleText: "text-white",
    subtitleText: "text-slate-400",
    labelMuted: "text-slate-500",
    valueMuted: "text-slate-300",
    chartArea: "border border-slate-800 bg-slate-950",
    sidebar: "border-r border-slate-800 bg-slate-900/90",
    toolBtnInactive: "text-slate-400 hover:text-white hover:bg-slate-800",
    tooltip: "bg-slate-800 border border-slate-700 text-white",
    divider: "border-t border-slate-800",
    deleteBtn: "text-slate-400 hover:text-red-400 hover:bg-red-500/10",
    chartBg: "bg-slate-950",
    toolbar: "border-b border-slate-800 bg-slate-900/40",
    segGroup: "bg-slate-800/60 border-slate-700/50",
    resInactive: "text-slate-400 hover:text-slate-200",
    separator: "bg-slate-800",
    chartTypeBtn:
      "text-slate-400 hover:text-white hover:bg-slate-800 hover:border-slate-700",
    indicatorLabel: "text-slate-500",
    indicatorBtn:
      "text-slate-300 bg-slate-800 hover:bg-slate-700 border-slate-700",
    indicatorActiveBtn: "bg-indigo-500 text-white shadow-sm border-indigo-500",
    themeBtn:
      "bg-amber-500/10 text-amber-400 hover:bg-amber-500/20 border-amber-500/20",
    sseBox: "text-slate-500 bg-slate-900/60 border-slate-800",
  },
};

// ===================== CANVAS THEMES =====================

const CANVAS_THEMES = {
  light: {
    grid: {
      horizontal: { color: "#d1d5db", size: 0.5 },
      vertical: { color: "#d1d5db", size: 0.5 },
    },
    candle: {
      bar: {
        upColor: "#16a34a",
        downColor: "#dc2626",
        noChangeColor: "#888888",
      },
      priceMark: {
        high: { color: "#6b7280", textSize: 10 },
        low: { color: "#6b7280", textSize: 10 },
      },
    },
    xAxis: {
      axisLine: { color: "#d1d5db" },
      tickText: { color: "#6b7280", size: 11 },
    },
    yAxis: {
      axisLine: { color: "#d1d5db" },
      tickText: { color: "#6b7280", size: 11 },
    },
    crosshair: {
      horizontal: { color: "#9ca3af", lineStyle: "dashed" },
      vertical: { color: "#9ca3af", lineStyle: "dashed" },
    },
    separator: { color: "#e5e7eb" },
    bg: "#ffffff",
  },
  dark: {
    grid: {
      horizontal: { color: "#64748b", size: 0.5 },
      vertical: { color: "#64748b", size: 0.5 },
    },
    candle: {
      bar: {
        upColor: "#22c55e",
        downColor: "#ef4444",
        noChangeColor: "#888888",
      },
      priceMark: {
        high: { color: "#64748b", textSize: 10 },
        low: { color: "#64748b", textSize: 10 },
      },
    },
    xAxis: {
      axisLine: { color: "#334155" },
      tickText: { color: "#94a3b8", size: 11 },
    },
    yAxis: {
      axisLine: { color: "#334155" },
      tickText: { color: "#94a3b8", size: 11 },
    },
    crosshair: {
      horizontal: { color: "#64748b", lineStyle: "dashed" },
      vertical: { color: "#64748b", lineStyle: "dashed" },
    },
    separator: { color: "#1e293b" },
    bg: "#020617",
  },
};

// ===================== HELPER =====================

function periodToSeconds(period) {
  const mult = { minute: 60, hour: 3600, day: 86400 };
  return (mult[period?.type] || 3600) * (period?.span || 1);
}

// ===================== CHART CLASS =====================

export default class Chart {
  constructor({
    broker = "binance",
    symbol = "BTCUSDT",
    title = "",
    subtitle = "",
    resolution = "1H",
    theme = "dark",
  } = {}) {
    // --- Props ---
    this.broker = broker;
    this.symbol = symbol;
    this.title = title;
    this.subtitle = subtitle;
    this.activeResolution = resolution;
    this.theme = theme;

    /** @type {import('klinecharts').Chart | null} */
    this.chart = null;
    this.stopRealtime = null;
    this.currentBar = null;

    // --- Display state (component template đọc trực tiếp) ---
    this.currentPrice = 0;
    this.priceChange = 0;
    this.priceChangePercent = 0;
    this.high24h = 0;
    this.low24h = 0;
    this.volume24h = 0;

    /** @type {Map<string, string>} Tên indicator → paneId */
    this.activeIndicators = new Map();

    /** Callback(name, activeIndicators) — gọi mỗi khi indicator state thay đổi */
    this.onIndicatorChange = null;

    // ==================== RESOLUTIONS ====================

    /** @type {Map<string, {value:string, label:string, period:object, getTimeRange:function}>} */
    this._resolutions = new Map();

    const _defineRes = (value, label, period, getTimeRange) => {
      this._resolutions.set(value, { value, label, period, getTimeRange });
    };

    _defineRes("1m", "1m", { type: "minute", span: 1 }, (now) => ({
      from: now - 4 * 3600,
      to: now,
      limit: 500,
    }));
    _defineRes("5m", "5m", { type: "minute", span: 5 }, (now) => ({
      from: now - 24 * 3600,
      to: now,
      limit: 500,
    }));
    _defineRes("1H", "1H", { type: "hour", span: 1 }, (now) => ({
      from: now - 30 * 24 * 3600,
      to: now,
      limit: 500,
    }));
    _defineRes("4H", "4H", { type: "hour", span: 4 }, (now) => ({
      from: now - 90 * 24 * 3600,
      to: now,
      limit: 500,
    }));
    _defineRes("1D", "1D", { type: "day", span: 1 }, (now) => ({
      from: now - 365 * 24 * 3600,
      to: now,
      limit: 500,
    }));

    // ==================== INDICATORS ====================

    /** @type {Map<string, {name:string, label:string, paneId:string, create?:function, remove?:function}>} */
    this._indicators = new Map();

    const _defineInd = (name, label, paneId, opts = {}) => {
      this._indicators.set(name, {
        name,
        label,
        paneId: paneId ?? "candle_pane",
        create: opts.create || null,
        remove: opts.remove || null,
      });
    };

    _defineInd("VOL", "Volume", "pane_volume", {
      create: (chart, paneId) => {
        chart.createIndicator({ name: "VOL" }, false, {
          id: paneId,
          height: 100,
        });
      },
      remove: (chart, paneId) => {
        chart.removeIndicator(paneId, "VOL");
      },
    });
    _defineInd("MA", "MA");
    _defineInd("EMA", "EMA");
    _defineInd("BOLL", "Bollinger Bands");
    _defineInd("RSI", "RSI", "pane_rsi");
    _defineInd("MACD", "MACD", "pane_macd");
  }

  /** Instance getters — thuận tiện cho template */
  get resolutions() {
    return Array.from(this._resolutions.values());
  }

  get indicators() {
    return Array.from(this._indicators.values());
  }

  get classes() {
    return TAILWIND_THEMES[this.theme] || TAILWIND_THEMES["dark"];
  }

  mount(container) {
    if (!container) return false;
    this.chart = init(container);
    if (!this.chart) return false;

    this.chart.setDataLoader(this._buildDataLoader());
    this.chart.setSymbol({
      ticker: this.symbol,
      pricePrecision: 2,
      volumePrecision: 4,
    });
    this.chart.setPeriod(this._resolutions.get(this.activeResolution).period);

    this.chart.setStyles(CANVAS_THEMES[this.theme]);

    // Bật VOL indicator mặc định
    this.toggleIndicator("VOL");

    return true;
  }

  destroy() {
    this.unsubscribeRealtime();
    if (this.chart) {
      dispose(this.chart);
      this.chart = null;
    }
  }

  setTheme(theme) {
    if (theme !== "light" && theme !== "dark") return;
    this.theme = theme;
    if (this.chart) {
      this.chart.setStyles(CANVAS_THEMES[theme]);
    }
  }

  setResolution(res) {
    if (res === this.activeResolution) return;
    this.activeResolution = res;
    const def = this._resolutions.get(res);
    if (this.chart && def) this.chart.setPeriod(def.period);
  }

  changeChartType(type) {
    if (this.chart) this.chart.setStyles({ candle: { type } });
  }

  toggleIndicator(name, paneId) {
    if (!this.chart) return;
    const def = this._indicators.get(name);
    if (!def) return;

    if (this.activeIndicators.has(name)) {
      const existingPaneId = this.activeIndicators.get(name);
      if (def.remove) {
        def.remove(this.chart, existingPaneId);
      } else {
        this.chart.removeIndicator(existingPaneId, name);
      }
      this.activeIndicators.delete(name);
    } else {
      const targetPaneId = paneId || def.paneId;
      if (def.create) {
        def.create(this.chart, targetPaneId);
      } else {
        this.chart.createIndicator({ name }, true, { id: targetPaneId });
      }
      this.activeIndicators.set(name, targetPaneId);
    }

    if (this.onIndicatorChange) {
      this.onIndicatorChange(name, new Map(this.activeIndicators));
    }
  }

  // ---------- Internal helpers ----------

  _calculateFetchRange(type, timestamp, defaultRange) {
    const now = Math.floor(Date.now() / 1000);
    if (type === "init" || type === "backward") {
      return {
        timeFrom:
          type === "backward" && timestamp
            ? Math.floor(timestamp / 1000)
            : defaultRange.from,
        timeTo: now,
      };
    }
    return {
      timeFrom: defaultRange.from,
      timeTo: timestamp ? Math.floor(timestamp / 1000) : defaultRange.to,
    };
  }

  _getLatestBarFromChart(barTimestamp) {
    if (!this.chart) return null;
    const dataList = this.chart.getDataList();
    if (dataList && dataList.length > 0) {
      const last = dataList[dataList.length - 1];
      if (last.timestamp === barTimestamp) return { ...last };
    }
    return null;
  }

  _getBarTimestamp(tickTimestamp) {
    const def = this._resolutions.get(this.activeResolution);
    const spanSeconds = def ? periodToSeconds(def.period) : 3600;
    const spanMs = spanSeconds * 1000;
    return Math.floor(tickTimestamp / spanMs) * spanMs;
  }

  _aggragateTickToBar(tick, barTimestamp) {
    const { price, quantity } = tick;
    if (!this.currentBar || this.currentBar.timestamp !== barTimestamp) {
      this.currentBar = this._getLatestBarFromChart(barTimestamp);
    }
    if (!this.currentBar || this.currentBar.timestamp !== barTimestamp) {
      this.currentBar = {
        timestamp: barTimestamp,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: quantity,
      };
    } else {
      this.currentBar.close = price;
      if (price > this.currentBar.high) this.currentBar.high = price;
      if (price < this.currentBar.low) this.currentBar.low = price;
      this.currentBar.volume += quantity;
    }
    return this.currentBar;
  }

  unsubscribeRealtime() {
    if (this.stopRealtime) this.stopRealtime();
  }

  _updateStats(data) {
    if (!data || !data.length) return;
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

    // Cập nhật trực tiếp — $state proxy bắt được
    this.currentPrice = latest.close;
    this.priceChange = prev ? latest.close - prev.close : 0;
    this.priceChangePercent =
      prev && prev.close ? ((latest.close - prev.close) / prev.close) * 100 : 0;
    this.high24h = maxH > -Infinity ? maxH : 0;
    this.low24h = minL < Infinity ? minL : 0;
    this.volume24h = volSum;
  }

  _buildDataLoader() {
    return {
      getBars: async ({ type, timestamp, callback }) => {
        const def = this._resolutions.get(this.activeResolution);
        const defaultRange = def.getTimeRange(Math.floor(Date.now() / 1000));
        const { timeFrom, timeTo } = this._calculateFetchRange(
          type,
          timestamp,
          defaultRange,
        );

        const data = await getHistoryCandles(
          this.broker,
          this.symbol,
          this.activeResolution,
          timeFrom,
          timeTo,
          defaultRange.limit,
        );
        this._updateStats(data);
        callback(data, {
          backward: type === "init",
          forward: data.length >= defaultRange.limit,
        });
      },

      subscribeBar: ({ callback }) => {
        const sseUrl = `${API_V1_ENDPOINT}/ohcl/last-price/${this.broker}/${this.symbol}`;
        const es = new EventSource(sseUrl);
        this.currentBar = null;

        es.addEventListener("tick", (event) => {
          try {
            const payload = JSON.parse(event.data);
            if (!payload?.tick) return;

            const barTimestamp = this._getBarTimestamp(payload.tick.timestamp);
            const updatedBar = this._aggragateTickToBar(
              payload.tick,
              barTimestamp,
            );

            // Cập nhật trực tiếp — $state proxy bắt được
            this.currentPrice = payload.tick.price;
            this.priceChange = updatedBar.close - updatedBar.open;
            this.priceChangePercent = updatedBar.open
              ? ((updatedBar.close - updatedBar.open) / updatedBar.open) * 100
              : 0;

            callback(updatedBar);
          } catch (e) {
            console.error("SSE parse error:", e);
          }
        });

        this.stopRealtime = () => {
          es.close();
          this.stopRealtime = null;
        };
      },

      unsubscribeBar: () => {
        this.unsubscribeRealtime();
      },
    };
  }
}
