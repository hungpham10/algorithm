/**
 * DrawingToolManager — quản lý các công cụ vẽ (overlay) trên chart
 * Các công cụ được định nghĩa trong constructor, cho phép hiện/tắt theo ý muốn
 */

export default class Draw {
  /**
   * @param {import('klinecharts').Chart | null} chart — klinecharts instance (có thể null, set sau qua setChart)
   */
  constructor(chart = null) {
    this._chart = chart;
    this._activeTool = "cursor";
    this._selectedOverlayId = null;
    this._pendingOverlayId = null;

    /** Callback(state) — được gọi mỗi khi activeTool hoặc selectedOverlayId thay đổi */
    this.onChange = null;

    /** Định nghĩa các công cụ vẽ — mỗi tool có thể được hiện/tắt */
    this._tools = [
      { id: "cursor", label: "Con trỏ (Chọn/Dịch)", icon: "🎯", visible: true },
      {
        id: "horizontalStraightLine",
        label: "Đường ngang (S/R)",
        icon: "▬",
        visible: true,
      },
      {
        id: "segment",
        label: "Đoạn thẳng (Trendline)",
        icon: "╱",
        visible: true,
      },
      {
        id: "straightLine",
        label: "Trendline vô hạn",
        icon: "↗",
        visible: true,
      },
      { id: "rayLine", label: "Tia (Ray)", icon: "⇗", visible: true },
      { id: "priceLine", label: "Đường giá", icon: "💰", visible: true },
      // rect: không phải built-in overlay của klinecharts, cần registerOverlay nếu muốn hỗ trợ
      {
        id: "parallelStraightLine",
        label: "Biên giao động (Channel)",
        icon: "≡",
        visible: true,
      },
      { id: "priceChannelLine", label: "Kênh giá", icon: "⫼", visible: true },
      {
        id: "fibonacciLine",
        label: "Fibo thoái lui",
        icon: "☰",
        visible: true,
      },
    ];
  }

  // ===================== Tool visibility control =====================

  /** Danh sách công cụ đang hiển thị */
  get tools() {
    return this._tools.filter((t) => t.visible);
  }

  /** Danh sách tất cả công cụ (kể cả ẩn) */
  get allTools() {
    return this._tools;
  }

  /** Hiện/tắt một công cụ theo id */
  setToolVisibility(toolId, visible) {
    const tool = this._tools.find((t) => t.id === toolId);
    if (tool) tool.visible = visible;
  }

  /** Ẩn một công cụ */
  hideTool(toolId) {
    this.setToolVisibility(toolId, false);
  }

  /** Hiện một công cụ */
  showTool(toolId) {
    this.setToolVisibility(toolId, true);
  }

  // ===================== Core =====================

  /** Gán chart mới (dùng khi mount xong mới có chart) */
  setChart(chart) {
    this._chart = chart;
  }

  get activeTool() {
    return this._activeTool;
  }
  get selectedOverlayId() {
    return this._selectedOverlayId;
  }

  _notify() {
    if (this.onChange) {
      this.onChange({
        activeTool: this._activeTool,
        selectedOverlayId: this._selectedOverlayId,
      });
    }
  }

  /** Chọn/vẽ một công cụ */
  selectTool(toolId) {
    this._activeTool = toolId;
    if (toolId === "cursor") {
      this._notify();
      return;
    }
    if (!this._chart) return;

    // Huỷ thao tác vẽ trước đó nếu đang vẽ dở
    if (this._pendingOverlayId) {
      try {
        this._chart.removeOverlay({ id: this._pendingOverlayId });
      } catch (_) {}
      this._pendingOverlayId = null;
    }

    // Dùng ID duy nhất để có thể remove sau này
    const sessionId = `__draw_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
    this._chart.createOverlay({
      name: toolId,
      id: sessionId,
      onDrawEnd: () => {
        if (this._pendingOverlayId === sessionId) {
          this._pendingOverlayId = null;
        }
        this._activeTool = "cursor";
        this._notify();
      },
      onSelected: (ev) => {
        this._selectedOverlayId = ev.overlay.id;
        this._notify();
      },
      onDeselected: () => {
        this._selectedOverlayId = null;
        this._notify();
      },
    });
    this._pendingOverlayId = sessionId;
    this._notify();
  }

  /** Xoá overlay đang được chọn */
  deleteSelected() {
    if (!this._chart || !this._selectedOverlayId) return;
    this._chart.removeOverlay({ id: this._selectedOverlayId });
    this._selectedOverlayId = null;
    this._notify();
  }

  /** Xoá tất cả overlay */
  clearAll() {
    if (!this._chart) return;
    this._chart.removeOverlay();
    this._pendingOverlayId = null;
    this._activeTool = "cursor";
    this._selectedOverlayId = null;
    this._notify();
  }

  /** Xử lý phím tắt */
  handleKeydown(event) {
    if (
      (event.key === "Delete" || event.key === "Backspace") &&
      this._selectedOverlayId
    ) {
      event.preventDefault();
      this.deleteSelected();
      return;
    }
    if (event.key === "Escape") {
      // Huỷ overlay đang vẽ dở nếu có
      if (this._pendingOverlayId) {
        try {
          this._chart?.removeOverlay({ id: this._pendingOverlayId });
        } catch (_) {}
        this._pendingOverlayId = null;
      }
      this._activeTool = "cursor";
      this._selectedOverlayId = null;
      this._notify();
    }
  }
}
