/**
 * @class       : utils
 * @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
 * @created     : Saturday May 02, 2026 08:57:57 +07
 * @description : utils
 */

export function formatVN(val) {
  if (val === null || val === undefined || val === "" || isNaN(val)) {
    return "---";
  }

  return Number(val).toLocaleString('vi-VN');
}

export function formatDiff(val) {
  if (val === null || val === undefined || val === "") {
    return "---";
  }

  const hasNotArrow = String(val).includes('●');
  const hasArrowUp = String(val).includes('▲');
  const hasArrowDown = String(val).includes('▼');
  const numericValue = String(val).replace(/[▲▼●\s]/g, '');

  if (numericValue === "" || isNaN(numericValue)) {
    return val;
  }

  const formattedNum = Number(numericValue).toLocaleString('vi-VN');

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
  const diffNum = parseFloat(String(diffStr).replace(/[^\d.-]/g, ''));
  const currNum = parseFloat(String(currentPrice));
  if (isNaN(diffNum) || isNaN(currNum)) return "0%";
  const isDown = String(diffStr).includes('▼') || String(diffStr).includes('-');
  const finalDiff = isDown ? -Math.abs(diffNum) : Math.abs(diffNum);
  const yesterday = currNum - finalDiff;
  if (yesterday <= 0) return "0%";
  const percent = (finalDiff / yesterday) * 100;
  return (percent > 0 ? "+" : "") + percent.toFixed(2) + "%";
};
