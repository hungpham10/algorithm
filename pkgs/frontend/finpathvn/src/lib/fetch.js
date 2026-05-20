/**
 * @class       : fetch
 * @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
 * @created     : Saturday May 02, 2026 09:43:24 +07
 * @description : fetch
 */

import { Prices } from './schema.js';

export const API_V2_ENDPOINT = "https://lighttrading.pp.ua/api/investing/v2";
export const API_V1_ENDPOINT = "https://lighttrading.pp.ua/api/investing/v1";

export async function fetchPriceByProductId(eventName, productIds, degree) {
  if (!productIds.length) {
    return;
  }

  try {
    const response = await fetch(
      `${API_V2_ENDPOINT}/prices/by-product/${productIds.join(',')}?degree=${degree}`
    );

    if (!response.ok) {
      throw new Error("Network response was not ok");
    }

    window.dispatchEvent(new CustomEvent(eventName, {
      detail: new Prices(await response.json()),
    }));
  } catch (error) {
    console.error(`Fetch price by products ${productIds}:`, error);
  }
}

export async function fetchCandlesticks(eventName, broker, symbol, resolution, from, to) {
  try {
    const response = await fetch(
      `${API_V1_ENDPOINT}/ohcl/candles/${broker}/${symbol}?resolution=${resolution}&from=${from}&to=${to}&limit=0`
    );

    if (!response.ok) {
      throw new Error("Network response was not ok");
    }

    const json = await response.json();

    window.dispatchEvent(new CustomEvent(eventName, {
      detail: {
        candlesticks: json.ohcl.map(item => ({
          timestamp: item.t * 1000,
          open: item.o,
          high: item.h,
          low: item.l,
          close: item.c,
          volume: item.v
        })),
        resolution: resolution,
        symbol: symbol,
        from: from,
        to: to,
      }
    }));
  } catch (error) {
    console.error(`Fetch candlesticks of ${broker}/${symbol} in resolution ${resolution} failed:`, error);
  }
}
