/**
 * @class       : utils
 * @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
 * @created     : Saturday May 02, 2026 08:57:57 +07
 * @description : utils
 */

export function formatMoney(value) {
  const num = parseFloat(value);
  if (isNaN(num)) return value;
  return num.toLocaleString("vi-VN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export function formatVN(val) {
  if (val === null || val === undefined || val === "" || isNaN(val)) {
    return "---";
  }

  return Number(val).toLocaleString("vi-VN");
}

export function formatDiff(val) {
  if (val === null || val === undefined || val === "") {
    return "---";
  }

  const hasNotArrow = String(val).includes("●");
  const hasArrowUp = String(val).includes("▲");
  const hasArrowDown = String(val).includes("▼");
  const numericValue = String(val).replace(/[▲▼●\s]/g, "");

  if (numericValue === "" || isNaN(numericValue)) {
    return val;
  }

  const formattedNum = Number(numericValue).toLocaleString("vi-VN");

  if (!hasArrowUp && !hasArrowDown) {
    if (numericValue > 0) {
      return `▲ ${formattedNum}`;
    } else if (numericValue < 0) {
      return `▼ ${formattedNum}`;
    } else {
      return `● ${formattedNum}`;
    }
  }

  if (hasNotArrow) {
    return `● ${formattedNum}`;
  } else if (hasArrowUp) {
    return `▲ ${formattedNum}`;
  } else if (hasArrowDown) {
    return `▼ ${formattedNum}`;
  } else {
    return formattedNum;
  }
}

export function calcPercentInitial(diffStr, currentPrice) {
  if (!diffStr || !currentPrice) return "0%";
  const diffNum = parseFloat(String(diffStr).replace(/[^\d.-]/g, ""));
  const currNum = parseFloat(String(currentPrice));
  if (isNaN(diffNum) || isNaN(currNum)) return "0%";
  const isDown = String(diffStr).includes("▼") || String(diffStr).includes("-");
  const finalDiff = isDown ? -Math.abs(diffNum) : Math.abs(diffNum);
  const yesterday = currNum - finalDiff;
  if (yesterday <= 0) return "0%";
  const percent = (finalDiff / yesterday) * 100;
  return (percent > 0 ? "+" : "") + percent.toFixed(2) + "%";
}

export async function getIP() {
  const ipResponse = await fetch("https://api.ipify.org?format=json");
  if (!ipResponse.ok) {
    throw new Error("Không thể lấy IP");
  }

  const ipData = await ipResponse.json();
  return ipData.ip;
}

export function removeVietnameseTones(str) {
  return str
    .normalize("NFD")
    .replace(/[\u0300-\u036f]/g, "")
    .replace(/đ/g, "d")
    .replace(/Đ/g, "D")
    .trim();
}

export function formatPrice(val) {
  if (!val && val !== 0) return "—";

  return val.toLocaleString("en-US", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  });
}

export function formatVolume(val) {
  if (!val && val !== 0) return "—";
  if (val >= 1000000) return (val / 1000000).toFixed(2) + "M";
  if (val >= 1000) return (val / 1000).toFixed(2) + "K";
  return val.toFixed(2);
}

/**
 * Helpers — Các hàm tính toán dùng chung cho indicators
 */

/** Đổi resolution string sang số giờ */
export function resolutionToIntervalHours(res) {
  const match = res.match(/^(\d+)([mHdWM])$/);
  if (!match) return 1;
  const num = parseInt(match[1]);
  const unit = match[2];
  if (unit === "m") return num / 60;
  if (unit === "H") return num;
  if (unit === "d") return num * 24;
  return 1;
}

/** Simple Moving Average */
export function sma(data, period) {
  const result = [];
  for (let i = period - 1; i < data.length; i++) {
    let sum = 0;
    for (let j = 0; j < period; j++) sum += data[i - j].close;
    result.push({ time: data[i].time, value: sum / period });
  }
  return result;
}

/** Exponential Moving Average */
export function ema(data, period) {
  const result = [];
  const k = 2 / (period + 1);
  let ema = data.slice(0, period).reduce((s, d) => s + d.close, 0) / period;
  for (let i = 0; i < data.length; i++) {
    if (i >= period) ema = (data[i].close - ema) * k + ema;
    if (i >= period - 1) result.push({ time: data[i].time, value: ema });
  }
  return result;
}

/** HSL color từ intensity (0→đỏ, 1→xanh) */
export function hsla(intensity, isCenter) {
  const hue = Math.round((1 - intensity) * 240);
  return isCenter
    ? `hsla(${hue}, 78%, 46%, 0.9)`
    : `hsla(${hue}, 60%, 50%, 0.6)`;
}
