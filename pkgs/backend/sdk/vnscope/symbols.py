import requests as rq
import polars as pl
import typing as tp
import numpy as np
import datetime as dt

from .models import CandleStick


class Symbols:
    _session: rq.Session = rq.Session()
    _base_url: str = (
        "https://lighttrading.pp.ua"  # Default; can be set via class method or instance
    )

    @classmethod
    def configure_base_url(cls, url: str) -> None:
        cls._base_url = url

    def __init__(self, base_url: str = None):
        if base_url:
            self._base_url = base_url

    def _get_symbols(self, broker: str, product: str) -> tp.List[str]:
        base_url = getattr(self, "_base_url", self._base_url)
        resp = self._session.get(
            f"{base_url}/api/investing/v1/ohcl/symbols/{broker}/{product}"
        )
        if resp.status_code == 200:
            data = resp.json()
            return data.get("symbols", [])
        return []

    def _fetch_ohcl(
        self, broker: str, symbol: str, resolution: str, from_ts: int, to_ts: int
    ) -> tp.List[CandleStick]:
        base_url = getattr(self, "_base_url", self._base_url)
        resp = self._session.get(
            f"{base_url}/api/investing/v1/ohcl/{broker}/{symbol}?resolution={resolution}&from={from_ts}&to={to_ts}&limit=0"
        )
        if resp.status_code != 200:
            raise ValueError(f"HTTP {resp.status_code}: {resp.text}")

        data = resp.json()
        if "error" in data:
            raise ValueError(f"API error: {data['error']}")

        ohcl_arr = data.get("ohcl", [])
        datapoints = []
        for candle in ohcl_arr:
            t = int(candle.get("t", 0))
            if t < from_ts or t > to_ts:
                continue
            datapoints.append(
                CandleStick(
                    t=t,
                    o=float(candle.get("o", 0)),
                    h=float(candle.get("h", 0)),
                    l=float(candle.get("l", 0)),
                    c=float(candle.get("c", 0)),
                    v=int(candle.get("v", 0)),
                )
            )
        return datapoints

    def crypto(self) -> tp.List[str]:
        return self._get_symbols("crypto", "spot")

    def etf(self) -> tp.List[str]:
        return self._get_symbols("stock", "etf")

    def futures(self) -> tp.List[str]:
        return self._get_symbols("futures", "futures")

    def midcap(self) -> tp.List[str]:
        return self._get_symbols("stock", "midcap")

    def penny(self) -> tp.List[str]:
        return self._get_symbols("stock", "penny")

    def vn30(self) -> tp.List[str]:
        return self._get_symbols("stock", "vn30")

    def vn100(self) -> tp.List[str]:
        return self._get_symbols("stock", "vn100")

    def industry(self, name) -> tp.List[str]:
        return self._get_symbols("stock", name)

    def cw(self) -> pl.DataFrame:
        base_url = getattr(self, "_base_url", self._base_url)
        resp = self._session.get(f"{base_url}/api/investing/v1/ohcl/symbols/stock/cw")
        if resp.status_code == 200:
            data = resp.json().get("cws")
            data = resp.json()
            if "error" in data:
                raise ValueError(f"API error: {data['error']}")
            cws = data.get("cws", [])

            # Assuming the API returns a direct JSON array of CWInfo objects
            # Each item has keys: "code", "underlyingAsset", "exercisePrice", "exerciseRatio", "lastTradingDate"
            df_data = {
                "symbol": [item.get("StockCode", "") for item in cws],
                "underlying": [item.get("underlyingAsset", "") for item in cws],
                "exercise_price": [int(item.get("ExcercisePrice", 0)) for item in cws],
                "exercise_ratio": [item.get("ExcerciseRatio", "") for item in cws],
                "last_trading_date": [item.get("LastTradingDate", "") for item in cws],
            }
            return pl.DataFrame(df_data)
        return pl.DataFrame(
            schema={
                "symbol": pl.Utf8,
                "underlying": pl.Utf8,
                "exercise_price": pl.UInt64,
                "exercise_ratio": pl.Utf8,
                "last_trading_date": pl.Utf8,
            }
        )

    def price(
        self,
        symbol: str,
        broker: str,
        resolution: str,
        from_date: str = "",
        to_date: str = "",
        from_ts: int = 0,
        to_ts: int = 0,
    ) -> pl.DataFrame:
        if from_ts == 0:
            from_dt = dt.datetime.strptime(from_date, "%Y-%m-%d").replace(
                tzinfo=dt.timezone.utc
            )
            from_ts = int(from_dt.timestamp())
        if to_ts == 0:
            to_dt = dt.datetime.strptime(to_date, "%Y-%m-%d").replace(
                tzinfo=dt.timezone.utc
            )
            to_ts = int(to_dt.timestamp())

        datapoints = self._fetch_ohcl(broker, symbol, resolution, from_ts, to_ts)

        df_data = {
            "Date": [dt.datetime.fromtimestamp(it.t) for it in datapoints],
            "Open": [it.o for it in datapoints],
            "High": [it.h for it in datapoints],
            "Close": [it.c for it in datapoints],
            "Low": [it.l for it in datapoints],
            "Volume": [float(it.v) for it in datapoints],
        }
        return pl.DataFrame(df_data)

    def log_return(
        self,
        symbol: str,
        broker: str,
        resolution: str,
        from_date: str = "",
        to_date: str = "",
        from_ts: int = 0,
        to_ts: int = 0,
    ) -> pl.DataFrame:
        if from_ts == 0 and from_date:
            from_dt = dt.datetime.strptime(from_date, "%Y-%m-%d").replace(
                tzinfo=dt.timezone.utc
            )
            from_ts = int(from_dt.timestamp())
        if to_ts == 0 and to_date:
            to_dt = dt.datetime.strptime(to_date, "%Y-%m-%d").replace(
                tzinfo=dt.timezone.utc
            )
            to_ts = int(to_dt.timestamp())
        datapoints = self._fetch_ohcl(
            broker,
            symbol,
            resolution,
            from_ts,
            to_ts,
        )
        df_data = {
            "Date": [dt.datetime.fromtimestamp(it.t) for it in datapoints],
            "Close": [it.c for it in datapoints],
        }
        df = pl.DataFrame(df_data)
        df = df.with_columns(
            (pl.col("Close") / pl.col("Close").shift(1)).log().alias("LogReturn")
        )
        return df.drop("Close").drop_nulls()

    def heatmap(
        self,
        symbol: str,
        broker: str,
        resolution: str,
        now: int,
        lookback: int,
        overlap: int,
        number_of_levels: int,
        interval_in_hour: int,
    ) -> tp.Tuple[
        np.ndarray,
        tp.List[float],
        tp.List[tp.Tuple[int, int, int]],
        tp.List[tp.Tuple[int, int]],
    ]:
        base_url = getattr(self, "_base_url", self._base_url)
        url = (
            f"{base_url}/api/investing/v1/ohcl/{broker}/{symbol}/heatmap?"
            f"resolution={resolution}&now={now}&lookback={lookback}&"
            f"overlap={overlap}&number_of_levels={number_of_levels}&"
            f"interval_in_hour={interval_in_hour}"
        )
        resp = self._session.get(url)
        if resp.status_code != 200:
            raise ValueError(f"HTTP {resp.status_code}: {resp.text}")
        data = resp.json()
        if "error" in data:
            raise ValueError(f"API error: {data['error']}")
        else:
            data = data.get("heatmap")

        data_arr = np.array(data.get("heatmap", []), dtype=float)
        levels = [float(v) for v in data.get("levels", [])]
        ranges = [
            (int(r[0]), int(r[1]), int(r[2]))
            for r in data.get("ranges", [])
            if len(r) == 3
        ]
        timelines = [
            (int(r[0]), int(r[1])) for r in data.get("timelines", []) if len(r) == 2
        ]
        return (
            data_arr,
            levels,
            ranges,
            timelines,
        )

    def history(
        self, symbols: tp.List[str], broker: str, resolution: str, lookback: int
    ) -> pl.DataFrame:
        to_ts = int(dt.datetime.now(dt.timezone.utc).timestamp())
        if resolution == "1D":
            from_ts = to_ts - 24 * 60 * 60 * lookback
        elif resolution == "1W":
            from_ts = to_ts - 7 * 24 * 60 * 60 * lookback
        else:
            raise ValueError(f"Not support resolution `{resolution}`")

        def fetch_candles(symbol: str) -> tp.List[CandleStick]:
            try:
                return self._fetch_ohcl(broker, symbol, resolution, from_ts, to_ts)
            except ValueError:
                return []

        with ThreadPoolExecutor() as executor:
            datapoints = list(executor.map(fetch_candles, symbols))

        max_candles = max(len(candles) for candles in datapoints) if datapoints else 0
        if max_candles == 0:
            return pl.DataFrame({"symbol": symbols})

        df_data = {"symbol": symbols}
        for i in range(max_candles):
            o_values = [
                candles[i].o if i < len(candles) else None for candles in datapoints
            ]
            h_values = [
                candles[i].h if i < len(candles) else None for candles in datapoints
            ]
            c_values = [
                candles[i].c if i < len(candles) else None for candles in datapoints
            ]
            l_values = [
                candles[i].l if i < len(candles) else None for candles in datapoints
            ]
            v_values = [
                float(candles[i].v) if i < len(candles) else None
                for candles in datapoints
            ]
            df_data[f"o_day_{i+1}"] = o_values
            df_data[f"h_day_{i+1}"] = h_values
            df_data[f"c_day_{i+1}"] = c_values
            df_data[f"l_day_{i+1}"] = l_values
            df_data[f"v_day_{i+1}"] = v_values

        return pl.DataFrame(df_data)
