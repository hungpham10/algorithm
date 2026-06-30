/**
 * Indicators — Module quản lý indicator
 *
 * Mỗi indicator là một file riêng trong thư mục indicators/,
 * export theo format { name, label, category, params, create, destroy }
 *
 * File này tổng hợp tất cả indicator để re-export.
 */

import { MA } from "./ma.js";
import { EMA } from "./ema.js";
import { BOLL } from "./boll.js";
import { RSI } from "./rsi.js";
import { MACD } from "./macd.js";
import { HEATMAP } from "./heatmap.js";

/** Object chứa tất cả định nghĩa indicator, key = name */
export const INDICATOR_DEFS = {
  MA,
  EMA,
  BOLL,
  RSI,
  MACD,
  HEATMAP,
};

/** Lấy danh sách indicator (dùng cho toolbar) */
export function getIndicatorList() {
  return Object.values(INDICATOR_DEFS).map((def) => ({
    name: def.name,
    label: def.label,
    category: def.category,
    params: def.params,
  }));
}

/** Lấy params default cho một indicator */
export function getDefaultParams(name) {
  const def = INDICATOR_DEFS[name];
  if (!def) return {};
  const params = {};
  for (const p of def.params) {
    params[p.name] = p.default;
  }
  return params;
}
