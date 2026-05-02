/**
 * @class       : fetch
 * @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
 * @created     : Saturday May 02, 2026 09:43:24 +07
 * @description : fetch
 */

import { Prices } from './schema.js';

export const API_V2_ENDPOINT = "/api/investing/v2";

export async function fetchPriceByProductId(eventName, productIds, degree) {
  if (!productIds.length) {
    return;
  }

  try {
    const response = await fetch(
      `${API_V2_ENDPOINT}/prices/by-product/${products.join(',')}?degree=${degree}`
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
