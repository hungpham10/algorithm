/**
 * @class       : render
 * @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
 * @created     : Saturday May 02, 2026 09:14:02 +07
 * @description : render
 */

import { formatVN, formatDiff, } from './utils.js';

export const GRAPHQL_ENDPOINT = "https://lighttrading.pp.ua/api/investing/v2/astra-render";

export class FetchIndex {
  constructor(data) {
    const calculatePercent = (diffStr, currentPrice) => {
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


    const goldSummary = data?.goldSummary?.[0] || {};
    const largeMarketGold = data?.largeMarketGold || [];

    this._summary = {
      vn: {
        buy: formatVN(goldSummary.buy),
        sell: formatVN(goldSummary.sell),
        diffBuy: calculatePercent(goldSummary.diffBuy, goldSummary.buy),
        diffSell: calculatePercent(goldSummary.diffSell, goldSummary.sell),
      },
      world: {
        price: "-",
        diff: "-",
        percent: "0%",
        convertedVnd: "0"
      }
    };

    this._tableLargecap = largeMarketGold.reduce((acc, item, index) => {
      acc[index + 1] = {
        ...item,
        formattedBuy: formatVN(item.buy),
        formattedSell: formatVN(item.sell),
        percentBuy: calculatePercent(item.diffBuy, item.buy),
        percentSell: calculatePercent(item.diffSell, item.sell)
      };
      return acc;
    }, {});

    this._productIds = [
      goldSummary.product,
      ...largeMarketGold.map(i => i.product)
    ]
    .filter(Boolean);
  }

  get tableLargecap() {
    return this._tableLargecap;
  }

  get listLargecap() {
    return Object.values(this._tableLargecap);
  }

  get tableLargecapEntries() {
    if (!this._tableLargecap) {
      return [];
    }

    return Object.entries(this._tableLargecap);
  }

  get productIds() {
    return this._productIds;
  }

  get summary() {
    return this._summary;
  }

  static async load() {
    const response = await fetch(GRAPHQL_ENDPOINT, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        query: `
          query GetGoldData {
            goldSummary: goldMarketList(scopes: [1], after: 0, limit: 1, degree: 1000000) {
              product, buy, sell, diffBuy, diffSell, yesterdayBuy, yesterdaySell
            }
            largeMarketGold: goldMarketList(scopes: [1, 2], after: 0, limit: 10, degree: 1000000) {
              product, description, buy, sell, diffBuy, diffSell, yesterdayBuy, yesterdaySell, trend, trendData
            }
          }
        `
      }),
    });

    const responseJson = await response.json();
    return new FetchIndex(responseJson.data);
  }
}
