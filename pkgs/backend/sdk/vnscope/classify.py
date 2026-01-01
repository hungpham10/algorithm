import matplotlib.pyplot as plt
import numpy as np
import polars as pl
import pandas as pd
import mplfinance as mpf
import seaborn as sns

from .symbols import Symbols
from .core import market


class ClassifyVolumeProfile:
    def __init__(
        self,
        base_url=None,
        now=None,
        resolution="1D",
        lookback=120,
        interval_in_hour=24,
    ):
        from datetime import datetime, timezone, timedelta

        self.symbols = Symbols(base_url)

        if now is None:
            self.now = int((datetime.now(timezone.utc) + timedelta(days=1)).timestamp())
        else:
            try:
                # Parse the now string (e.g., "2025-01-01") to a datetime object
                now_dt = datetime.strptime(now, "%Y-%m-%d")
                # Ensure the datetime is timezone-aware (UTC)
                now_dt = now_dt.replace(tzinfo=timezone.utc)
                # Convert to timestamp
                self.now = int(now_dt.timestamp())
            except ValueError as e:
                raise ValueError(
                    "Invalid 'now' format. Use 'YYYY-MM-DD' (e.g., '2025-01-01')"
                )

        self.resolution = resolution
        self.lookback = lookback
        self.interval_in_hour = interval_in_hour

    def plot_heatmap_with_candlestick(
        self,
        symbol,
        broker,
        number_of_levels,
        overlap_days,
        excessive=1.1,
        top_n=3,
        enable_heatmap=False,
        enable_inverst_ranges=False,
        fill_ranges=True,
        show_poc_only=False,
        max_breaking_trend=4,
        max_thush_trend=2,
    ):
        import pandas as pd
        import numpy as np
        import matplotlib.pyplot as plt
        import mplfinance as mpf
        import seaborn as sns
        from datetime import datetime

        if self.resolution == "1D":
            interval = 24 * 60 * 60
        elif self.resolution == "1H":
            interval = 60 * 60
        elif self.resolution == "1W":
            interval = 7 * 27 * 60 * 60
        else:
            return None

        # === 1. Lấy dữ liệu ===
        from_time = datetime.fromtimestamp(
            self.now - self.lookback * interval
        ).strftime("%Y-%m-%d")
        to_time = datetime.fromtimestamp(self.now).strftime("%Y-%m-%d")

        candlesticks = self.symbols.price(
            symbol, broker, self.resolution, from_time, to_time
        ).to_pandas()

        consolidated, levels, ranges, timelines = self.symbols.heatmap(
            symbol,
            broker,
            self.resolution,
            self.now,
            self.lookback,
            overlap_days,
            number_of_levels,
            self.interval_in_hour,
        )

        price_df = candlesticks.copy()
        price_df["Date"] = pd.to_datetime(price_df["Date"])
        price_df.set_index("Date", inplace=True)
        price_df = price_df.sort_index()

        if len(price_df) < overlap_days:
            print("Không đủ dữ liệu")
            return

        heatmap_dates = price_df.index[0 : max(0, len(price_df) - overlap_days)]

        # === 2. Indicators cơ bản ===
        period = overlap_days
        price_df["SMA"] = price_df["Close"].rolling(period).mean()
        price_df["Upper"] = (
            price_df["SMA"] + price_df["Close"].rolling(period).std() * 2
        )
        price_df["Lower"] = (
            price_df["SMA"] - price_df["Close"].rolling(period).std() * 2
        )
        price_df["High_Vol"] = (
            price_df["Volume"] > price_df["Volume"].rolling(period).mean() * excessive
        )
        price_df["Marker"] = np.where(
            price_df["High_Vol"], price_df["High"] * 1.01, np.nan
        )

        max_idx = price_df[price_df["High_Vol"]]["Volume"].idxmax()
        price_df["Max_Marker"] = np.nan
        if pd.notna(max_idx):
            price_df.loc[max_idx, "Max_Marker"] = price_df.loc[max_idx, "High"] * 1.025

        apds = [
            mpf.make_addplot(price_df["SMA"], color="blue", width=1.2),
            mpf.make_addplot(price_df["Upper"], color="red", alpha=0.5),
            mpf.make_addplot(price_df["Lower"], color="green", alpha=0.5),
            mpf.make_addplot(
                price_df["Marker"],
                type="scatter",
                marker="^",
                markersize=80,
                color="lime",
            ),
            mpf.make_addplot(
                price_df["Max_Marker"],
                type="scatter",
                marker="*",
                markersize=220,
                color="red",
            ),
        ]

        # === 3. Xử lý từng range - chuẩn pro ===
        if enable_inverst_ranges:
            ranges.reverse()
            timelines.reverse()

        colors = sns.color_palette("husl", top_n)
        fill_artists = []

        for i, (poc_idx, low_idx, high_idx) in enumerate(ranges[:top_n]):
            if i >= len(timelines):
                continue

            col_start, col_end = timelines[i]
            nominal_start = heatmap_dates[col_start]
            nominal_end = heatmap_dates[col_end]

            val = levels[low_idx]
            vah = levels[high_idx]
            poc = levels[poc_idx]
            color = colors[i]

            # === TÌM NGÀY ĐẦU TIÊN GIÁ CHẠM VÀO VÙNG (first touch) ===
            data_initial = price_df[
                (price_df.index >= nominal_start) & (price_df.index <= nominal_end)
            ]
            touch_initial = (data_initial["High"] >= val) & (data_initial["Low"] <= vah)
            actual_start_date = (
                data_initial[touch_initial].index[0]
                if touch_initial.any()
                else nominal_start
            )

            # === TÌM NGÀY CUỐI CÙNG GIÁ CÒN TRONG/CHẠM VÙNG ===
            data_from_start = price_df[price_df.index >= actual_start_date]

            # Ưu tiên: nếu còn nến nằm HOÀN TOÀN trong VA → kéo đến hiện tại
            fully_inside = (data_from_start["Low"] >= val) & (
                data_from_start["High"] <= vah
            )

            if fully_inside.any():
                actual_end_date = price_df.index[-1]
                inc_thush_trend = max_thush_trend
                values = fully_inside.values
                dates = fully_inside.index
                sideway = values[0]
                thrush = 0
                cnt = 0

                for j in range(len(values)):
                    if values[j] != sideway:
                        cnt += 1

                        if cnt >= max_breaking_trend:
                            sideway = values[j]
                            cnt = max_breaking_trend
                            thrush = 0
                    else:
                        is_thrused = False

                        for k in range(cnt):
                            if data_from_start["Low"].values[j - cnt] <= val:
                                is_thrused = True
                            elif data_from_start["High"].values[j] >= vah:
                                is_thrused = True
                            else:
                                continue
                            break

                        if cnt > 0 and is_thrused:
                            thrush += 1
                        cnt = 0

                    if sideway and thrush >= inc_thush_trend:
                        inc_thush_trend += 1
                        actual_end_date = dates[j]
            else:
                # Nếu không còn nằm gọn → lấy ngày cuối cùng còn CHẠM vùng
                touch_mask = (data_from_start["High"] >= val) & (
                    data_from_start["Low"] <= vah
                )

                if touch_mask.any():
                    actual_end_date = data_from_start[touch_mask].index[-1]
                else:
                    actual_end_date = nominal_end  # fallback hiếm gặp

            # === Tạo mask thời gian thực tế để vẽ ===
            final_mask = (price_df.index >= actual_start_date) & (
                price_df.index <= actual_end_date
            )
            if not final_mask.any():
                continue

            x_dates = price_df.index[final_mask]

            # === Tô vùng Value Area ===
            if fill_ranges:
                fill_artists.append(
                    {
                        "x": x_dates,
                        "y1": val,
                        "y2": vah,
                        "color": color,
                        "alpha": 0.25,
                        "label": f"VA {i + 1} ({actual_start_date.date()} → {actual_end_date.date()})",
                    }
                )

            # === Vẽ POC kéo dài ===
            poc_series = pd.Series(np.nan, index=price_df.index)
            poc_series[final_mask] = poc
            apds.append(
                mpf.make_addplot(
                    poc_series,
                    color=color,
                    alpha=0.95,
                    width=2,
                    label=f"POC {i + 1}",
                )
            )

            # === Vẽ VAL / VAH (nếu không chỉ POC) ===
            if not show_poc_only:
                for price, style in [(val, "--"), (vah, "--")]:
                    line = pd.Series(np.nan, index=price_df.index)
                    line[final_mask] = price
                    apds.append(
                        mpf.make_addplot(
                            line,
                            color=color,
                            alpha=0.8,
                            linestyle=style,
                        )
                    )

        # === 4. Vẽ biểu đồ ===
        fig, axes = mpf.plot(
            price_df,
            type="candle",
            style="charles",
            addplot=apds,
            volume=True,
            volume_panel=1,
            panel_ratios=(5, 1),
            figsize=(21, 11),
            tight_layout=True,
            show_nontrading=False,
            title=f"{symbol} • Smart Volume Profile Ranges (First Touch → Broken/Current)",
            returnfig=True,
        )

        ax = axes[0]

        # Vẽ vùng VA thông minh
        for art in fill_artists:
            ax.fill_between(
                art["x"],
                art["y1"],
                art["y2"],
                color=art["color"],
                alpha=art["alpha"],
                label=art["label"],
            )

        # Legend đẹp, loại bỏ trùng lặp
        handles, labels = ax.get_legend_handles_labels()
        by_label = dict(zip(labels, handles))
        ax.legend(
            by_label.values(),
            by_label.keys(),
            loc="upper left",
            fontsize=10,
            framealpha=0.9,
            ncol=1,
        )

        plt.show()

    def calculate_beta_between_index_and_symbols(
        self,
        index,
        symbols,
        broker,
        resolution,
        overlap,
    ):
        df_index = self.symbols.log_return(
            index,
            broker,
            resolution,
            from_ts=self.now - self.lookback * 24 * 60 * 60,
            to_ts=self.now,
        )

        def calculate_beta(symbol, df_index):
            df = self.symbols.log_return(
                symbol,
                broker,
                resolution,
                from_ts=self.now - self.lookback * 24 * 60 * 60,
                to_ts=self.now,
            ).join(
                df_index,
                on="Date",
                how="inner",
            )
            betas = []
            timestamps = []

            for i in range(len(df) - overlap + 1):
                df_sliced = df.slice(i, overlap)

                cov = df_sliced.select(
                    pl.cov("LogReturn", "LogReturn_right").alias("correlation")
                ).row(0)[0]
                var = df_sliced.select(
                    pl.var("LogReturn_right").alias("correlation")
                ).row(0)[0]
                timestamp = df_sliced["Date"].max()

                betas.append(cov / var)
                timestamps.append(timestamp)
            return {
                "beta": betas,
                "timestamp": timestamps,
            }

        return (
            pl.DataFrame({"symbol": symbols})
            .with_columns(
                pl.struct(["symbol"])
                .map_elements(
                    lambda row: calculate_beta(row["symbol"], df_index),
                    strategy="threading",
                    return_dtype=pl.Struct(
                        {
                            "beta": pl.List(pl.Float64),
                            "timestamp": pl.List(pl.Datetime),
                        }
                    ),
                )
                .alias("output"),
            )
            .with_columns(
                pl.struct(["output"])
                .map_elements(
                    lambda row: row["output"]["beta"],
                    return_dtype=pl.List(pl.Float64),
                )
                .alias("beta"),
                pl.struct(["output"])
                .map_elements(
                    lambda row: row["output"]["timestamp"],
                    return_dtype=pl.List(pl.Datetime),
                )
                .alias("timestamp"),
            )[("symbol", "beta", "timestamp")]
        )

    def detect_possible_reverse_point(
        self,
        symbols,
        broker,
        number_of_levels,
        overlap_days,
    ):
        # Hàm helper để gọi heatmap và extract info
        def get_heatmap_info(symbol: str):
            (_, levels, ranges, timelines) = self.symbols.heatmap(
                symbol,
                broker,
                self.resolution,
                self.now,
                self.lookback,
                overlap_days,
                number_of_levels,
                self.interval_in_hour,
            )
            centers = []
            begins = []
            ends = []
            for (center, begin, end) in ranges:
                centers.append(center)
                begins.append(begin)
                ends.append(end)
            return {
                "levels": levels,
                "centers": centers,
                "begins": begins,
                "ends": ends,
            }

        def possible_down_to(price, heatmap):
            ends = heatmap["ends"]
            begins = heatmap["begins"]
            levels = heatmap["levels"]
            centers = heatmap["centers"]

            # next centers according price
            mapping = sorted(
                [i for i in range(0, len(centers))],
                key=lambda i: levels[centers[i]],
            )
            blocks = [
                (i, levels[begins[i]], levels[centers[i]], levels[ends[i]])
                for i in mapping
            ]

            for (p, (i, begin, center, end)) in enumerate(blocks):
                if (begin < price < end) or (
                    (blocks[i - 1][2] if i > 0 else 0.0) < price < begin
                ):
                    if price >= center * 1.07:
                        if len(blocks) == i + 1:
                            return center

                    for q in range(p, 1, -1):
                        if blocks[q][0] > blocks[q - 1][0]:
                            return (blocks[q - 1][2] + blocks[q - 1][3]) / 2.0

                    for q in range(p, 0, -1):
                        if blocks[q][2] < price:
                            return (blocks[q][1] + blocks[q][2]) / 2.0

            if blocks[-1][2] < price:
                return blocks[-1][2]
            elif blocks[0][2] < price:
                return blocks[0][2]
            elif blocks[0][1] < price:
                return blocks[0][1]
            return 0.0

        def possible_distributed_phase(price, heatmap):
            ends = heatmap["ends"]
            begins = heatmap["begins"]
            levels = heatmap["levels"]
            centers = heatmap["centers"]

            # next centers according price
            mapping = sorted(
                [i for i in range(0, len(centers))],
                key=lambda i: levels[centers[i]],
            )
            blocks = [
                (i, levels[begins[i]], levels[centers[i]], levels[ends[i]])
                for i in mapping
            ]

            # RISK FILTER 1: if price is outside blocks, consider to do nothing
            min_price = blocks[0][1]
            max_price = blocks[-1][3]

            if price < min_price:
                return None
            if price > max_price:
                return None
            cnv_cnt = 0
            inv_cnt = 0
            shift = 0
            flow = None
            for p, (i, begin, center, end) in enumerate(blocks):
                if (begin < price < end) or (
                    (blocks[i - 1][2] if i > 0 else 0.0) < price < begin
                ):
                    for q in range(p, 0, -1):
                        if blocks[q][0] > blocks[q - 1][0]:
                            if flow is None or flow is False:
                                shift += 1
                                flow = True
                            if shift > 2:
                                break
                            inv_cnt += 1
                        else:
                            if flow is None or flow is False:
                                shift += 1
                                flow = False
                            if shift > 2:
                                break
                            cnv_cnt += 1

                    if p > 0:
                        if cnv_cnt > 0:
                            return 1.0 * cnv_cnt / p
                        else:
                            return -1.0 * inv_cnt / p
                    else:
                        break
            return None

        def max_distance_between_centers(heatmap):
            levels = heatmap["levels"]
            centers = heatmap["centers"]

            mapping = sorted(
                [i for i in range(0, len(centers))],
                key=lambda i: levels[centers[i]],
            )

            return max(
                [
                    levels[centers[mapping[i]]] - levels[centers[mapping[i - 1]]]
                    for i in range(1, len(mapping))
                ]
            )

        def calculate_bollinger_bank(symbol, window=20, std_multiplier=2.0):
            from datetime import datetime, timedelta

            # Estimate time range
            from_time = datetime.fromtimestamp(
                self.now - self.lookback * 24 * 60 * 60,
            ).strftime("%Y-%m-%d")
            to_time = datetime.fromtimestamp(self.now).strftime("%Y-%m-%d")

            price_df = self.symbols.price(
                symbol,
                broker,
                self.resolution,
                from_time,
                to_time,
            ).to_pandas()
            price_df["Date"] = pd.to_datetime(price_df["Date"])
            price_df = price_df.sort_values("Date").reset_index(drop=True)
            price_df.set_index("Date", inplace=True)

            price_df["SMA"] = (
                price_df["Close"].rolling(window=window, min_periods=1).mean()
            )
            price_df["STD"] = (
                price_df["Close"].rolling(window=window, min_periods=1).std()
            )

            return (2.0 * std_multiplier * price_df["STD"] / price_df["SMA"]).to_list()

        return (
            market(symbols)[("symbol", "price")]
            .with_columns(
                pl.col("symbol")
                .map_elements(
                    get_heatmap_info,
                    strategy="threading",
                    return_dtype=pl.Struct(
                        {
                            "levels": pl.List(pl.Float64),
                            "centers": pl.List(pl.Int64),
                            "begins": pl.List(pl.Int64),
                            "ends": pl.List(pl.Int64),
                        }
                    ),
                )
                .alias("heatmap"),
                pl.col("symbol")
                .map_elements(
                    calculate_bollinger_bank,
                    strategy="threading",
                    return_dtype=pl.List(pl.Float64),
                )
                .alias("bolingger_width"),
            )
            .with_columns(
                pl.struct(["price", "heatmap"])
                .map_elements(
                    lambda row: possible_down_to(
                        row["price"],
                        row["heatmap"],
                    ),
                    return_dtype=pl.Float64,
                )
                .alias("possible_down_to"),
                pl.struct(["price", "heatmap"])
                .map_elements(
                    lambda row: possible_distributed_phase(
                        row["price"],
                        row["heatmap"],
                    ),
                    return_dtype=pl.Float64,
                )
                .alias("possible_distributed_phase"),
                pl.struct(["heatmap"])
                .map_elements(
                    lambda row: max_distance_between_centers(row["heatmap"]),
                    return_dtype=pl.Float64,
                )
                .alias("max_distance_between_centers"),
            )[
                (
                    "symbol",
                    "price",
                    "possible_down_to",
                    "possible_distributed_phase",
                    "max_distance_between_centers",
                )
            ]
        )
