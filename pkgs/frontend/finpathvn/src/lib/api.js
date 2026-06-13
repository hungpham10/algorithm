/**
 * @class       : api
 * @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
 * @created     : Saturday May 02, 2026 08:55:35 +07
 * @description : api
 */

import { API_V2_ENDPOINT } from './fetch.js';
import { formatMoney } from './utils.js';

/**
 * Lấy danh sách giá vàng theo khu vực/vùng miền
 * @param {string} region - Tên tỉnh/thành phố (VD: "Hồ Chí Minh")
 * @param {string|null} cursor - Con trỏ phân trang tiếp theo
 * @param {number} pageSize - Số lượng bản ghi trên một trang
 * @returns {Promise<{items: Array, nextCursor: string|null}>}
 */
export async function getGoldPricesByRegion(region, cursor = null, pageSize = 5) {
  try {
    let url = `${API_V2_ENDPOINT}/prices/by-province/${encodeURIComponent(region)}?limit=${pageSize}&degree=1000000`;
    if (cursor) url += `&after=${cursor}`;

    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`HTTP error! status: ${response.status}`);
    }

    const rawData = await response.json();
    let items = [];
    let nextCursor = null;

    if (Array.isArray(rawData) && rawData.length > 0) {
      const priceData = rawData[0].prices.data || {};

      items = Object.keys(priceData).map(brandName => ({
        product: brandName,
        buy: priceData[brandName]?.buy ?? 0,
        sell: priceData[brandName]?.sell ?? 0,
        store: brandName
      }));

      nextCursor = rawData.prices?.next_after || null;
    }

    return { items, nextCursor };

  } catch (error) {
    console.error("Lỗi khi call API giá vàng:", error);
    return { items: [], nextCursor: null };
  }
}


// Danh sách ánh xạ tên ngoại tệ cố định
const currencyNames = {
  USD: "Đô la Mỹ", EUR: "Euro", GBP: "Bảng Anh", JPY: "Yên Nhật",
  AUD: "Đô la Úc", SGD: "Đô la Singapore", THB: "Baht Thái Lan",
  CAD: "Đô la Canada", CHF: "Franc Thụy Sĩ", HKD: "Đô la Hồng Kông",
  CNY: "Nhân dân tệ", DKK: "Krone Đan Mạch", INR: "Rupee Ấn Độ",
  KRW: "Won Hàn Quốc", KWD: "Dinar Kuwait", MYR: "Ringgit Malaysia",
  NOK: "Krone Na Uy", RUB: "Rúp Nga", SAR: "Riyal Ả Rập Xê Út", SEK: "Krona Thụy Điển"
};

/**
 * Thu thập và xử lý tỷ giá ngoại tệ hiện tại cùng lịch sử xu hướng xu hướng 30 ngày.
 * @returns {Promise<{ exchangeData: Array, historyCache: Object, currentDate: string }>}
 */
export async function getExchangeRatesWithHistory() {
  try {
    let exchangeData = [];

    const today = new Date();
    const currentDate = today.toLocaleDateString('vi-VN') + " " + today.toLocaleTimeString('vi-VN', { hour: '2-digit', minute: '2-digit' });

    const historyCache = {};
    const dateStrings = [];

    // Tạo mảng 30 chuỗi ngày lịch sử
    for (let i = 0; i < 30; i++) {
      const d = new Date();
      d.setDate(today.getDate() - i);
      dateStrings.push(`${d.getFullYear()}-${d.getMonth() + 1}-${d.getDate()}`);
    }

    // Thực hiện call song song dữ liệu 30 ngày
    const fetchPromises = dateStrings.map(dateStr =>
      fetch(`${API_V2_ENDPOINT}/exchange-rate?date=${dateStr}`)
        .then(res => res.ok ? res.json() : null)
        .catch(() => null)
    );

    const allResults = await Promise.all(fetchPromises);

    // Xử lý ngược từ quá khứ (ngày 30) tiến dần về hiện tại (ngày 0) để tạo cache lịch sử đúng thứ tự thời gian
    for (let i = allResults.length - 1; i >= 0; i--) {
      const resData = allResults[i];
      if (resData && resData.query) {
        resData.query.forEach(item => {
          const cashVal = parseFloat(item.cash);
          const sellVal = parseFloat(item.sell);
          if (item.currencyCode && !isNaN(cashVal) && cashVal !== 0) {
            if (!historyCache[item.currencyCode]) {
              historyCache[item.currencyCode] = [];
            }
            historyCache[item.currencyCode].push({
              date: dateStrings[i],
              cash: cashVal,
              sell: sellVal
            });
          }
        });
      }
    }

    // Trích xuất dữ liệu mới nhất (ngày hiện tại) làm bảng hiển thị chính
    const latestRes = allResults[0];

    if (latestRes && latestRes.query) {
      const validRates = latestRes.query.filter(item => {
        if (!item.cash) return false;
        const cashVal = parseFloat(item.cash);
        return !isNaN(cashVal) && cashVal !== 0;
      });

      exchangeData = validRates.map(item => ({
        code: item.currencyCode,
        name: currencyNames[item.currencyCode] || "Ngoại tệ khác",
        cash: formatMoney(item.cash),
        sell: formatMoney(item.sell)
      }));
    }

    return {
      exchangeData,
      historyCache,
      currentDate
    };

  } catch (error) {
    return { exchangeData: [], historyCache: {}, currentDate: "" };
  }
}
