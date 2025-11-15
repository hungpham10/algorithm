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
    ):
        from datetime import datetime, timedelta

        # Estimate time range
        from_time = datetime.fromtimestamp(
            self.now - self.lookback * 24 * 60 * 60,
        ).strftime("%Y-%m-%d")
        to_time = datetime.fromtimestamp(self.now).strftime("%Y-%m-%d")

        # Collect data using Symbols class
        candlesticks = self.symbols.price(
            symbol,
            broker,
            self.resolution,
            from_time,
            to_time,
        ).to_pandas()
        consolidated, levels, ranges, timelines = self.symbols.heatmap(
            symbol,
            broker,  # Use provided broker
            self.resolution,
            self.now,
            self.lookback,
            overlap_days,
            number_of_levels,
            self.interval_in_hour,
        )

        # Convert from_time and to_time to datetime for time axis
        start_date = datetime.strptime(from_time, "%Y-%m-%d")

        # Create time axis for heatmap (starting from the overlap_days to match
        # overlap)
        heatmap_dates = pd.date_range(
            start=start_date + timedelta(days=overlap_days),
            periods=consolidated.shape[1],
            freq="D",
        )

        # Create full time axis for price data
        price_dates = pd.date_range(
            start=start_date,
            periods=len(candlesticks),
            freq="D",
        )

        # Invert levels for low to high order on y-axis
        consolidated = np.flipud(consolidated)

        # Prepare candlestick data
        price_df = candlesticks.copy()
        price_df["Date"] = pd.to_datetime(price_df["Date"])
        price_df.set_index("Date", inplace=True)

        # Calculate Bollinger Bands
        period = overlap_days
        price_df["SMA"] = price_df["Close"].rolling(window=period).mean()
        price_df["STD"] = price_df["Close"].rolling(window=period).std()
        price_df["Upper Band"] = price_df["SMA"] + (price_df["STD"] * 2)
        price_df["Lower Band"] = price_df["SMA"] - (price_df["STD"] * 2)

        # Calculate MA of Volume
        volume_ma_period = overlap_days
        price_df["Volume_MA"] = (
            price_df["Volume"].rolling(window=volume_ma_period).mean()
        )

        # Identify candles where Volume > Volume_MA
        price_df["High_Volume"] = price_df["Volume"] > price_df["Volume_MA"] * excessive

        # Calculate deviation of Volume from Volume_MA
        price_df["Volume_Deviation"] = price_df["Volume"] - price_df["Volume_MA"]

        # Find the point with the maximum deviation where Volume > Volume_MA
        max_deviation_idx = price_df[price_df["High_Volume"]][
            "Volume_Deviation"
        ].idxmax()
        max_deviation_value = (
            price_df.loc[max_deviation_idx, "Volume_Deviation"]
            if pd.notna(max_deviation_idx)
            else None
        )

        # Create a series for markers (place markers above the high of candles
        # where volume > MA)
        price_df["Marker"] = np.where(
            price_df["High_Volume"], price_df["High"] * 1.01, np.nan
        )

        # Create a series for the max deviation marker
        price_df["Max_Deviation_Marker"] = np.nan
        if pd.notna(max_deviation_idx):
            price_df.loc[max_deviation_idx, "Max_Deviation_Marker"] = (
                price_df.loc[max_deviation_idx, "High"] * 1.02
            )  # Slightly higher for visibility

        # For integrated plotting, mpf.plot creates its own figure. To integrate
        # heatmap, plot separately or use returnfig=True
        # Here, we'll let mpf.plot create its own figure for candlestick +
        # volume, and plot heatmap separately if enabled
        if enable_heatmap:
            fig_heatmap, ax_heatmap = plt.subplots(
                figsize=(60, 24)
            )  # Increased size for heatmap
            # Plot heatmap with imshow
            im = ax_heatmap.imshow(
                consolidated,
                aspect="auto",
                interpolation="nearest",
                extent=[0, consolidated.shape[1] - 1, 0, len(levels) - 1],
            )
            ytick_indices = range(0, len(levels), 5)  # Show every 5th label
            ax_heatmap.set_yticks(ytick_indices)
            ax_heatmap.set_yticklabels(np.round(levels, 5)[ytick_indices])
            ax_heatmap.set_title(
                "Volume Profile Heatmap for {} ({})".format(symbol, self.resolution)
            )
            ax_heatmap.set_ylabel("Price Levels")
            ax_heatmap.set_xticks(
                range(0, len(heatmap_dates), max(1, len(heatmap_dates) // 10))
            )
            ax_heatmap.set_xticklabels([])
            plt.colorbar(im, ax=ax_heatmap, label="Volume")
            plt.tight_layout()  # Improve spacing
            plt.show()

        # Create a colormap for price range lines
        colors = sns.color_palette("husl", n_colors=top_n)

        # Add horizontal lines for Bollinger Bands and markers (with
        # consolidated labels to reduce legend clutter)
        apds = [
            mpf.make_addplot(price_df["SMA"], color="blue", width=1, label="SMA"),
            mpf.make_addplot(
                price_df["Upper Band"], color="red", width=1, label="Upper Band"
            ),
            mpf.make_addplot(
                price_df["Lower Band"],
                color="green",
                width=1,
                label="Lower Band",
            ),
            mpf.make_addplot(
                price_df["Marker"],
                type="scatter",
                marker="^",
                color="green",
                markersize=10,
                label="High Volume",
            ),
            mpf.make_addplot(
                price_df["Max_Deviation_Marker"],
                type="scatter",
                marker="*",
                color="red",
                markersize=10,
                label="Max Volume Deviation",
            ),
        ]

        if enable_inverst_ranges:
            ranges.reverse()

        # Add price range lines (begin, center, end), but only over the specific
        # timeline periods for each range
        for i, (center, begin, end) in enumerate(ranges):
            if i >= top_n:
                break
            color = colors[i % len(colors)]  # Select color from palette

            apds.extend(
                [
                    mpf.make_addplot(
                        pd.Series(levels[begin], index=price_df.index),
                        color=color,
                        linestyle="--",
                        width=0.5,
                        label=f"Range {i+1}",
                    ),
                    mpf.make_addplot(
                        pd.Series(levels[center], index=price_df.index),
                        color=color,
                        linestyle="--",
                        width=1.0,
                        label=f"Range {i+1} Center",
                    ),
                    mpf.make_addplot(
                        pd.Series(levels[end], index=price_df.index),
                        color=color,
                        linestyle="--",
                        width=0.5,
                        label=f"Range {i+1} End",
                    ),
                ]
            )

        # Plot candlestick with Bollinger Bands and horizontal lines (increased
        # figsize, adjusted volume panel, legend position)
        mpf.plot(
            price_df,
            type="candle",
            style="charles",
            show_nontrading=False,
            addplot=apds,  # Add Bollinger Bands and horizontal lines
            volume=True,
            volume_panel=1,  # Use panel 1 for volume
            panel_ratios=(3, 1),  # Allocate more space to main chart vs volume
            figsize=(12, 7),  # Increased figure size
            tight_layout=True,  # Improve overall spacing
            returnfig=False,
        )

        # Note: For full integration with custom subplots, consider using
        # mpf.plot with returnfig=True and manual subplot addition. This version
        # plots heatmap separately if enabled, and candlestick in its own figure

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

            for (p, (i, begin, center, end)) in enumerate(blocks):
                if (begin < price < end) or (
                    (blocks[i - 1][2] if i > 0 else 0.0) < price < begin
                ):
                    for q in range(p, 1, -1):
                        if blocks[q][0] > blocks[q - 1][0]:
                            return False
                    else:
                        return True
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
                    return_dtype=pl.Boolean,
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
