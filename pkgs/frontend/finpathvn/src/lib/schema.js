/**
 * @class       : schema
 * @author      : Hung Nguyen Xuan Pham (hung0913208@gmail.com)
 * @created     : Saturday May 02, 2026 09:43:47 +07
 * @description : schema
 */


export const PRICE_EVENT_NAME = "price-update";

export class Price {
  constructor(data) {
    const p = data.price || {};
    const d = p.diff || [];

    this._error = data.error || false;
    if (!this._error) {
      this._buy = Number(p.buy || 0);
      this._sell = Number(p.sell || 0);
      this._diffBuy = Number(d[0] || 0);
      this._diffSell = Number(d[1] || 0);
    }
  }

  get error() {
    return this._error
  }

  get spread() {
    return this._sell - this._buy;
  }

  get buy() {
    return this._buy;
  }

  get sell() {
    return this._sell;
  }

  get change_buy() {
    return this._diffBuy;
  }

  get change_sell() {
    return this._diffSell;
  }
}

export class Prices {
  constructor(rawPrices) {
    this.prices = rawPrices.map(item => new Price(item));
  }

  get count() {
    return this.prices.length;
  }

  getItem(index) {
    return this.prices[index] || null;
  }

  getAll() {
    return [...this.prices];
  }

  *[Symbol.iterator]() {
    for (const item of this.prices) {
      yield item;
    }
  }
}

export class Summary {
  constructor(data) {
    const calculatePercent = (diffStr, currentPrice) => {
      if (!diffStr || !currentPrice) {
        return "0%";
      }

      const diffNum = parseFloat(String(diffStr).replace(/[^\d.-]/g, ''));
      const currNum = parseFloat(String(currentPrice));

      if (isNaN(diffNum) || isNaN(currNum)) {
        return "0%";
      }

      const isDown = String(diffStr).includes('▼') || String(diffStr).includes('-');
      const finalDiff = isDown ? -Math.abs(diffNum) : Math.abs(diffNum);
      const yesterday = currNum - finalDiff;
      const percent = (finalDiff / yesterday) * 100;

      if (yesterday <= 0) {
        return "0%";
      }

      return (percent > 0 ? "+" : "") + percent.toFixed(2) + "%";
    };

    this._vietnam = {
      buy: formatVN(data.buy),
      sell: formatVN(data.sell),
      diffBuy: calculatePercent(data.diffBuy, data.buy),
      diffSell: calculatePercent(data.diffSell, data.sell),
    };

    this._world =  {
      price: "-",
      diff: "-",
      percent: "-",
      convertedVnd: "0",
    };
  }

  get vietnam() {
    return this._vietnam;
  }

  get world() {
    return this._world;
  }
}
