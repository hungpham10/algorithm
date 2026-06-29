/**
 * LightDraw — Drawing tools wrapper
 *
 * DrawingManager handles: rendering via IPrimitivePaneView,
 * anchor drag (mousedown on an anchor of selected drawing).
 *
 * WE handle:
 * - Drawing creation via subscribeClick (anchor collection)
 * - Preview via subscribeCrosshairMove
 * - Selection + body drag via container mousedown/move/up
 * - Boundary drag for ParallelChannel
 * - Keyboard (delete, escape)
 *
 * KEY DESIGN:
 * - Selection happens on mousedown (not mouseup), so user can
 *   click-drag in one motion — like TradingView.
 * - DrawingManager is ALWAYS in cursor/selection mode (setActiveTool(null)).
 *   We collect anchors ourselves.
 */
import { DrawingManager, getToolRegistry } from "lightweight-charts-drawing";
import { LineSeries } from "lightweight-charts";

const registry = getToolRegistry();

const TOOLS = [
  { id: "cursor", label: "Con trỏ", icon: "🎯" },
  { id: "horizontal-line", label: "Đường ngang", icon: "▬" },
  { id: "trend-line", label: "Trendline", icon: "↗" },
  { id: "vertical-line", label: "Đường dọc", icon: "↕" },
  { id: "fib-retracement", label: "Fibonacci", icon: "☰" },
  { id: "parallel-channel", label: "Kênh song song", icon: "≡" },
  { id: "rectangle", label: "Hình chữ nhật", icon: "▭" },
  { id: "circle", label: "Hình tròn", icon: "○" },
  { id: "arrow", label: "Mũi tên", icon: "➡" },
  { id: "text-annotation", label: "Chữ", icon: "T" },
];

const PREVIEW_STYLE = {
  color: "#6366f1",
  lineWidth: 2,
  lineStyle: 2, // Dashed
};

// Hit distance threshold in pixels
const HIT_THRESHOLD = 10;

export default class LightDraw {
  constructor() {
    this._manager = new DrawingManager();
    this._chart = null;
    this._series = null;
    this._container = null;
    this._attached = false;
    this._activeTool = "cursor";
    this._selectedId = null;
    this._unsubs = [];
    this.onChange = null;

    // Drawing creation state
    this._pendingType = null;
    this._pendingAnchors = [];
    this._previewSer = null;
    this._uid = 1;

    // rAF throttle for preview
    this._rafId = null;
    this._pendingPreview = null;

    // Drag state
    this._isDragging = false;
    this._dragMode = null; // 'body' | 'boundary'
    this._dragStartX = 0;
    this._dragStartY = 0;
    this._dragAnchors = [];

    // Bound handlers
    this._boundClick = null;
    this._boundCross = null;
    this._boundDown = null;
    this._boundMove = null;
    this._boundUp = null;
    this._boundKey = null;
  }

  get tools() {
    return TOOLS;
  }
  get activeTool() {
    return this._activeTool;
  }
  get selectedOverlayId() {
    return this._selectedId;
  }

  // ── Attach ──
  setChart(chart, container, series) {
    if (this._attached) return;
    this._chart = chart;
    this._series = series;
    this._container = container;

    // Attach manager in cursor/selection mode
    this._manager.attach(chart, series, container);
    this._manager.setActiveTool(null);

    // Chart subscribes for DRAWING CREATION only
    this._boundClick = this._onClick.bind(this);
    chart.subscribeClick(this._boundClick);

    this._boundCross = this._onCrosshair.bind(this);
    chart.subscribeCrosshairMove(this._boundCross);

    // Container events for SELECTION + DRAG
    this._boundDown = this._onMouseDown.bind(this);
    container.addEventListener("mousedown", this._boundDown);

    this._boundMove = this._onMouseMove.bind(this);
    container.addEventListener("mousemove", this._boundMove);

    this._boundUp = this._onMouseUp.bind(this);
    container.addEventListener("mouseup", this._boundUp);

    // DrawingManager events — track selection changes
    this._unsubs.push(
      this._manager.on("drawing:selected", (e) => {
        this._selectedId = e.drawingId || null;
        this._notify();
      }),
      this._manager.on("drawing:deselected", () => {
        this._selectedId = null;
        this._notify();
      }),
    );

    // Keyboard
    this._boundKey = (e) => this.handleKeydown(e);
    container.addEventListener("keydown", this._boundKey);

    this._attached = true;
  }

  destroy() {
    this._endDrag();
    this._cancelPreview();
    for (const u of this._unsubs) u();
    this._unsubs = [];
    if (this._chart) {
      if (this._boundClick) this._chart.unsubscribeClick(this._boundClick);
      if (this._boundCross)
        this._chart.unsubscribeCrosshairMove(this._boundCross);
    }
    if (this._container) {
      if (this._boundDown)
        this._container.removeEventListener("mousedown", this._boundDown);
      if (this._boundMove)
        this._container.removeEventListener("mousemove", this._boundMove);
      if (this._boundUp)
        this._container.removeEventListener("mouseup", this._boundUp);
      if (this._boundKey)
        this._container.removeEventListener("keydown", this._boundKey);
    }
    if (this._attached) {
      this._manager.detach();
      this._attached = false;
    }
    this._removePreviewSer();
    this._chart = null;
    this._series = null;
    this._container = null;
  }

  // ── Tool selection ──
  selectTool(toolId) {
    this._endDrag();
    this._cancelPreview();
    this._pendingAnchors = [];
    this._removePreviewSer();

    this._activeTool = toolId || "cursor";
    this._pendingType = toolId === "cursor" ? null : toolId;

    // Always keep manager in cursor mode — we handle creation
    this._manager.setActiveTool(null);
    this._manager.deselectAll();

    if (this._chart) {
      this._chart.applyOptions({
        handleScroll: { pressedMouseMove: toolId === "cursor" },
      });
    }

    console.log(`[LightDraw] Tool: "${toolId}"`);
    this._notify();
  }

  // ══════════════════════════════════════════════════
  // DRAWING CREATION  (subscribeClick / subscribeCrosshairMove)
  // ══════════════════════════════════════════════════

  _onClick(param) {
    if (!this._pendingType || !param.point || !this._series || !this._chart)
      return;

    const px = param.point.x;
    const py = param.point.y;

    // Don't place anchor on top of an existing drawing
    if (this._manager.hitTest({ x: px, y: py })) return;

    const time = this._chart.timeScale().coordinateToTime(px);
    const price = this._series.coordinateToPrice(py);
    if (time == null || price == null) return;

    const def = registry.get(this._pendingType);
    const needed = def ? def.requiredAnchors : 2;

    this._pendingAnchors.push({ time, price });

    // 1-anchor tools: finish immediately
    if (needed <= 1) {
      this._finishDrawing();
      return;
    }

    // After first anchor, show preview line
    if (this._pendingAnchors.length === 1) {
      this._createPreviewSer(time, price);
    }

    if (this._pendingAnchors.length >= needed) {
      this._finishDrawing();
    }

    this._notify();
  }

  /** Real-time preview from anchor[0] → cursor position */
  _onCrosshair(param) {
    if (
      !this._pendingType ||
      this._pendingAnchors.length !== 1 ||
      !this._previewSer
    )
      return;
    if (!param.point || param.point.x < 0 || param.point.y < 0) return;
    if (param.time == null) return;

    const price = this._series?.coordinateToPrice(param.point.y);
    if (price == null) return;

    this._pendingPreview = {
      anchor: this._pendingAnchors[0],
      time: param.time,
      price,
    };

    if (this._rafId == null) {
      this._rafId = requestAnimationFrame(() => {
        this._rafId = null;
        this._flushPreview();
      });
    }
  }

  _flushPreview() {
    const pp = this._pendingPreview;
    this._pendingPreview = null;
    if (!pp || !this._previewSer) return;

    // Sort by time ascending (v5 requires asc order)
    const a = { time: pp.anchor.time, value: pp.anchor.price };
    const b = { time: pp.time, value: pp.price };
    const data = a.time <= b.time ? [a, b] : [b, a];

    try {
      this._previewSer.setData(data);
    } catch (_) {}
  }

  _createPreviewSer(time, price) {
    this._removePreviewSer();
    try {
      this._previewSer = this._chart.addSeries(LineSeries, {
        color: PREVIEW_STYLE.color,
        lineWidth: PREVIEW_STYLE.lineWidth,
        lineStyle: PREVIEW_STYLE.lineStyle,
        lastValueVisible: false,
        priceLineVisible: false,
        autoscaleInfoProvider: () => null,
      });
      this._previewSer.setData([{ time, value: price }]);
    } catch (_) {}
  }

  _removePreviewSer() {
    if (this._previewSer && this._chart) {
      try {
        this._chart.removeSeries(this._previewSer);
      } catch (_) {}
      this._previewSer = null;
    }
  }

  _cancelPreview() {
    if (this._rafId != null) {
      cancelAnimationFrame(this._rafId);
      this._rafId = null;
    }
    this._pendingPreview = null;
  }

  _finishDrawing() {
    this._cancelPreview();
    this._removePreviewSer();

    const anchors = [...this._pendingAnchors];
    this._pendingAnchors = [];
    if (!anchors.length) return;

    const id = `draw-${this._uid++}`;
    const drawing = registry.createDrawing(
      this._pendingType,
      id,
      anchors,
      {},
      {},
    );
    if (drawing) {
      this._manager.addDrawing(drawing);
    }

    // Re-enable scroll after draw
    if (this._chart) {
      this._chart.applyOptions({ handleScroll: { pressedMouseMove: true } });
    }
  }

  // ══════════════════════════════════════════════════
  // SELECTION + DRAG  (mousedown / mousemove / mouseup)
  // ══════════════════════════════════════════════════

  _onMouseDown(e) {
    if (!this._chart || !this._series || !this._container) return;

    const rect = this._container.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    const viewport = this._buildViewport();
    if (!viewport) return;

    // ── DRAWING MODE: place anchors via subscribeClick (not here) ──
    if (this._activeTool !== "cursor") return;

    // ── CURSOR MODE: selection + drag ──

    // Hit-test via DrawingManager
    const hitDrawing = this._manager.hitTest({ x, y });

    if (hitDrawing && !hitDrawing.options.locked) {
      // Select immediately (on mousedown, not mouseup)
      this._manager.selectDrawing(hitDrawing.id);

      // Check anchor hit → let DrawingManager handle anchor drag
      const anchorIdx = hitDrawing.hitTestAnchor({ x, y }, viewport);
      if (anchorIdx !== null) return;

      // Check boundary hit (ParallelChannel)
      const boundary = this._hitTestChannelBoundary(
        hitDrawing,
        { x, y },
        viewport,
      );
      if (boundary) {
        this._startDrag(e, hitDrawing, "boundary");
        e.preventDefault();
        return;
      }

      // Check body hit → start body drag
      if (hitDrawing.testHit({ x, y }, viewport)) {
        this._startDrag(e, hitDrawing, "body");
        e.preventDefault();
        return;
      }

      return;
    }

    // Nothing hit → deselect
    if (this._selectedId) {
      this._manager.deselectAll();
    }
  }

  _onMouseMove(e) {
    if (!this._isDragging || !this._selectedId || !this._chart || !this._series)
      return;

    const drawing = this._manager.getDrawing(this._selectedId);
    if (!drawing) {
      this._endDrag();
      return;
    }

    const rect = this._container.getBoundingClientRect();
    const startX = this._dragStartX - rect.left;
    const startY = this._dragStartY - rect.top;
    const curX = e.clientX - rect.left;
    const curY = e.clientY - rect.top;

    // Convert pixel deltas to time/price
    const startTime = this._chart.timeScale().coordinateToTime(startX);
    const curTime = this._chart.timeScale().coordinateToTime(curX);
    const startPrice = this._series.coordinateToPrice(startY);
    const curPrice = this._series.coordinateToPrice(curY);

    if (
      startTime == null ||
      curTime == null ||
      startPrice == null ||
      curPrice == null
    )
      return;

    const dTime = curTime - startTime;
    const dPrice = startPrice - curPrice; // Y-axis inverted

    if (dTime === 0 && dPrice === 0) return;

    if (this._dragMode === "boundary") {
      // Only adjust anchor[2] price (parallel channel offset line)
      const newAnchors = this._dragAnchors.map((a) => ({ ...a }));
      if (newAnchors.length >= 3) {
        newAnchors[2] = {
          ...newAnchors[2],
          price: this._dragAnchors[2].price + dPrice,
        };
        try {
          drawing.setAnchors(newAnchors);
        } catch (_) {}
      }
    } else {
      // Body drag: shift all anchors by uniform delta
      const newAnchors = this._dragAnchors.map((a) => ({
        time: a.time + dTime,
        price: a.price + dPrice,
      }));
      try {
        drawing.setAnchors(newAnchors);
      } catch (_) {}
    }
  }

  _onMouseUp(_e) {
    if (this._isDragging) {
      this._endDrag();
    }
  }

  _startDrag(e, drawing, mode) {
    this._isDragging = true;
    this._dragMode = mode;
    this._dragStartX = e.clientX;
    this._dragStartY = e.clientY;
    this._dragAnchors = drawing.anchors.map((a) => ({ ...a }));

    // Disable chart scroll while dragging
    if (this._chart) {
      this._chart.applyOptions({ handleScroll: { pressedMouseMove: false } });
    }
  }

  _endDrag() {
    if (this._isDragging) {
      this._isDragging = false;
      this._dragMode = null;
      this._dragAnchors = [];
      if (this._chart) {
        this._chart.applyOptions({ handleScroll: { pressedMouseMove: true } });
      }
    }
  }

  /**
   * Hit-test the boundary lines of a ParallelChannel
   * Returns 'top' or 'bottom' if click is on a boundary line, null otherwise.
   */
  _hitTestChannelBoundary(drawing, point, viewport) {
    if (drawing.type !== "parallel-channel") return null;
    const anchors = drawing.anchors;
    if (anchors.length < 3) return null;

    const p0 = drawing.anchorToPixel(anchors[0], viewport);
    const p1 = drawing.anchorToPixel(anchors[1], viewport);
    const p2 = drawing.anchorToPixel(anchors[2], viewport);
    if (!p0 || !p1 || !p2) return null;

    // Line 1: through p0→p1
    // Line 2: through p2, parallel to p0→p1 (end = p2 + (p1 - p0))
    const dx = p1.x - p0.x;
    const dy = p1.y - p0.y;
    const p2End = { x: p2.x + dx, y: p2.y + dy };

    const dist1 = this._distToLine(point, p0, p1);
    const dist2 = this._distToLine(point, p2, p2End);

    if (dist1 < HIT_THRESHOLD) return "bottom";
    if (dist2 < HIT_THRESHOLD) return "top";

    return null;
  }

  /** Perpendicular distance from point to line segment */
  _distToLine(p, a, b) {
    const dx = b.x - a.x;
    const dy = b.y - a.y;
    const lenSq = dx * dx + dy * dy;
    if (lenSq === 0) return Math.hypot(p.x - a.x, p.y - a.y);
    const t = Math.max(
      0,
      Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / lenSq),
    );
    return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
  }

  /** Build viewport for hit-testing */
  _buildViewport() {
    if (!this._chart || !this._series || !this._container) return null;
    return {
      width: this._chart.timeScale().width(),
      height: this._container.clientHeight,
      timeScale: {
        coordinateToTime: (x) => this._chart.timeScale().coordinateToTime(x),
        timeToCoordinate: (t) => this._chart.timeScale().timeToCoordinate(t),
        logicalToCoordinate: (l) =>
          this._chart.timeScale().logicalToCoordinate(l),
      },
      priceScale: {
        coordinateToPrice: (y) => this._series.coordinateToPrice(y),
        priceToCoordinate: (p) => this._series.priceToCoordinate(p),
      },
    };
  }

  // ══════════════════════════════════════════════════
  // MANAGEMENT
  // ══════════════════════════════════════════════════

  clearAll() {
    this._endDrag();
    this._cancelPreview();
    this._pendingAnchors = [];
    this._removePreviewSer();
    this._manager.clearAll();
    this._selectedId = null;
    this._notify();
  }

  deleteSelected() {
    if (this._selectedId) {
      this._manager.removeDrawing(this._selectedId);
      this._selectedId = null;
      this._notify();
    }
  }

  handleKeydown(e) {
    if ((e.key === "Delete" || e.key === "Backspace") && this._selectedId) {
      e.preventDefault();
      this.deleteSelected();
      return;
    }
    if (e.key === "Escape") {
      if (this._pendingAnchors.length > 0) {
        this._cancelPreview();
        this._removePreviewSer();
        this._pendingAnchors = [];
        this._notify();
      } else {
        this.selectTool("cursor");
      }
    }
  }

  _notify() {
    this.onChange?.({
      activeTool: this._activeTool,
      selectedOverlayId: this._selectedId,
      collecting: this._pendingAnchors.length,
    });
  }
}
