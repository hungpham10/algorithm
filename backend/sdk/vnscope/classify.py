import matplotlib.pyplot as plt
import polars as pl
import numpy as np
import pandas as pd
import mplfinance as mpf


class ClassifyVolumeProfile:
    def __init__(self, now=None, resolution="1D", lookback=120, value_area_pct=0.7):
        from datetime import datetime, timezone, timedelta

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
        self.value_area_pct = value_area_pct

    def prepare_volume_profile(self, df_profile, number_of_levels):
        """Transform DataFrame into long format with price and volume per level.

        Args:
            df (pl.DataFrame, optional): Input DataFrame. Uses self.df if None.

        Returns:
            pl.DataFrame: Long-format DataFrame with columns [symbol, price, volume].
        """
        if df_profile is None:
            raise ValueError(
                "DataFrame must be provided either at initialization or as argument."
            )

        level_columns = [f"level_{i}" for i in range(number_of_levels)]

        # Calculate price step for each symbol
        df_profile = df_profile.with_columns(
            price_step=(pl.col("price_at_level_last") - pl.col("price_at_level_first"))
            / (number_of_levels - 1)
        )

        # Create a list of price levels for each symbol
        df_profile = df_profile.with_columns(
            prices=pl.struct(["price_at_level_first", "price_step"]).map_elements(
                lambda x: [
                    x["price_at_level_first"] + i * x["price_step"]
                    for i in range(number_of_levels)
                ],
                return_dtype=pl.List(pl.Float64),
            )
        )

        # Melt the DataFrame to long format
        df_long = df_profile.melt(
            id_vars=["symbol", "prices", "price_at_level_first", "price_at_level_last"],
            value_vars=level_columns,
            variable_name="level",
            value_name="volume",
        )

        # Extract the price for each level
        return df_long.with_columns(
            price=pl.col("prices").list.get(
                pl.col("level").str.extract(r"level_(\d+)").cast(pl.Int32)
            ),
            level=pl.col("level").str.extract(r"level_(\d+)").cast(pl.Int32),
        ).select(["symbol", "price", "volume", "level"])

    def calculate_poc_and_value_area(self, df_long):
        """Calculate Point of Control (POC) and Value Area (70% of volume) for each symbol.

        Args:
            df_long (pl.DataFrame): Long-format DataFrame with [symbol, price, volume].

        Returns:
            pl.DataFrame: DataFrame with [symbol, poc_price, poc_volume, vah, val, total_volume].
        """
        # Calculate POC (price with maximum volume)
        poc = (
            df_long.group_by("symbol")
            .agg(
                poc_price=pl.col("price")
                .filter(pl.col("volume") == pl.col("volume").max())
                .first(),
                poc_volume=pl.col("volume").max(),
            )
            .unique(subset=["symbol"])
        )

        # Calculate total volume
        total_volume = (
            df_long.group_by("symbol")
            .agg(total_volume=pl.col("volume").sum())
            .unique(subset=["symbol"])
        )

        # Calculate value area (70% of total volume)
        def value_area_calc(group):
            symbol = group["symbol"][0]
            volumes = group.sort("volume", descending=True).select(["price", "volume"])
            target_volume = group["volume"].sum() * self.value_area_pct

            # Use cumulative sum to find value area more efficiently
            volumes = volumes.with_columns(cumsum=pl.col("volume").cum_sum())
            value_area_rows = volumes.filter(pl.col("cumsum") <= target_volume)

            # Include the first row that exceeds target if value_area is empty
            if value_area_rows.is_empty():
                value_area_rows = volumes.head(1)
            elif value_area_rows.height < volumes.height:
                # Include the row that crosses the threshold
                next_row = volumes.filter(pl.col("cumsum") > target_volume).head(1)
                value_area_rows = pl.concat([value_area_rows, next_row])

            value_area_prices = value_area_rows["price"].to_list()

            return {
                "symbol": symbol,
                "vah": max(value_area_prices),
                "val": min(value_area_prices),
            }

        # Create value_area DataFrame
        value_area = pl.DataFrame(
            [value_area_calc(group) for _, group in df_long.group_by("symbol")]
        ).unique(subset=["symbol"])

        # Join POC, value area, and total volume
        result = (
            poc.join(value_area, on="symbol", how="inner")
            .join(total_volume, on="symbol", how="inner")
            .select(["symbol", "poc_price", "poc_volume", "vah", "val", "total_volume"])
        )

        return result

    def classify_volume_profile_shape(self, df_long, poc_va_df, min_peak_distance=0.0):
        """Classify the volume profile shape for each symbol.

        Args:
            df_long (pl.DataFrame): Long-format DataFrame with [symbol, price, volume].
            poc_va_df (pl.DataFrame): DataFrame with [symbol, poc_price, poc_volume, vah, val, total_volume].

        Returns:
            pl.DataFrame: DataFrame with [symbol, shape].
        """

        def classify_shape(group, poc, vah, val, total_volume, min_peak_distance):
            prices = group["price"]
            volumes = group["volume"]
            profile_high = prices.max()
            profile_low = prices.min()
            price_range = profile_high - profile_low
            poc_position = (poc - profile_low) / price_range if price_range > 0 else 0.5
            # Volume above and below POC
            lower_volume = group.filter(pl.col("price") < poc)["volume"].sum()
            upper_volume = group.filter(pl.col("price") > poc)["volume"].sum()
            threshold = total_volume / len(volumes) * 1.5

            # Identify peaks (volumes above threshold)
            peak_candidates = (
                group.filter(pl.col("volume") > threshold)
                .select(["price", "volume"])
                .sort("price")
            )
            if peak_candidates.is_empty():
                peaks = []
            else:
                # Group nearby peaks
                peaks = [
                    {
                        "price": peak_candidates["price"][0],
                        "volume": peak_candidates["volume"][0],
                    }
                ]
                last_peak_price = peak_candidates["price"][0]
                for price, volume in zip(
                    peak_candidates["price"][1:], peak_candidates["volume"][1:]
                ):
                    if price - last_peak_price >= min_peak_distance:
                        peaks.append({"price": price, "volume": volume})
                        last_peak_price = price

            shape = "Undefined"

            # Determine if peaks are balanced or skewed
            if len(peaks) >= 2:
                # Check if one peak is dominant (e.g., > 1.5x volume of others)
                peak_volumes = [p["volume"] for p in peaks]
                max_peak_volume = max(peak_volumes)
                max_peak_price = peaks[peak_volumes.index(max_peak_volume)]["price"]
                if max_peak_volume > 1.5 * sum(
                    v for v in peak_volumes if v != max_peak_volume
                ):
                    # Dominant peak suggests P-Shaped if skewed
                    if max_peak_price > poc and lower_volume / total_volume < 0.2:
                        shape = "P-Shaped"
                    if max_peak_price < poc and upper_volume / total_volume < 0.2:
                        shape = "b-Shaped"
                shape = "B-Shaped"

            # Existing logic for other shapes
            if (
                abs(poc_position - 0.5) < 0.2
                and lower_volume / total_volume > 0.2
                and upper_volume / total_volume > 0.2
            ):
                shape = "D-Shaped"
            if poc_position > 0.65 and lower_volume / total_volume < 0.2:
                shape = "P-Shaped"
            if poc_position < 0.35 and upper_volume / total_volume < 0.2:
                shape = "b-Shaped"
            if group["volume"].max() / total_volume < 0.05:
                shape = "I-Shaped"
            return shape, peaks

        # Initialize an empty list for results
        shapes = []

        # Iterate over each symbol group
        for symbol, group in df_long.group_by("symbol"):
            # Get POC, VAH, VAL, and total_volume for the symbol
            poc_data = poc_va_df.filter(pl.col("symbol") == symbol[0])
            if poc_data.is_empty():
                continue
            poc = poc_data["poc_price"][0]
            vah = poc_data["vah"][0]
            val = poc_data["val"][0]
            total_volume = poc_data["total_volume"][0]

            # Classify the shape for the group
            shape, peaks = classify_shape(
                group, poc, vah, val, total_volume, min_peak_distance
            )
            shapes.append(
                {
                    "symbol": symbol[0],
                    "shape": shape,
                    "peaks": peaks,
                }
            )

        # Convert results to DataFrame
        return pl.DataFrame(shapes)

    def join_with_current_market(self, df_profile, df_market):
        return (
            df_profile.join(
                df_market.rename(
                    {
                        "price": "current_price",
                    }
                ),
                on="symbol",
            )
            .with_columns(
                (pl.col("price") - pl.col("current_price")).abs().alias("price_diff"),
            )
            .with_columns(pl.col("price_diff").min().over(["symbol"]).alias("min_diff"))
            .filter(pl.col("price_diff") == pl.col("min_diff"))
            .drop(["price_diff", "min_diff"])
        )

    def detect_volume_price_divergence(self, df, window=3):
        price = df["Close"]
        volume = df["Volume"]

        def find_extrema(data, window):
            extrema = []
            if len(data) == 0:
                return extrema

            for i in range(window, len(data) - window):
                is_low = all(
                    data[i] <= data[i - j] for j in range(1, window + 1)
                ) and all(data[i] <= data[i + j] for j in range(1, window + 1))
                is_high = all(
                    data[i] >= data[i - j] for j in range(1, window + 1)
                ) and all(data[i] >= data[i + j] for j in range(1, window + 1))
                if is_low:
                    extrema.append((i, "low", data[i]))
                elif is_high:
                    extrema.append((i, "high", data[i]))
            extrema.append(
                (
                    len(data) - 1,
                    "low" if data[-1] < extrema[-1][2] else "high",
                    data[-1],
                )
            )
            return extrema

        price_extrema = find_extrema(price, window)

        ret = []
        j = 0
        k = 0

        for i in range(1, len(price_extrema)):
            if price_extrema[i][1] != price_extrema[j][1]:
                k = i

            if k != j and price_extrema[i][1] == price_extrema[j][1]:
                if volume[j] < volume[i - 1] and price[j] > price[i - 1]:
                    ret.append(
                        f"Bullish divergence at {df['Date'][j]} - {df['Date'][i - 1]}"
                    )
                elif volume[j] > volume[i - 1] and price[j] < price[i - 1]:
                    ret.append(
                        f"Berrish divergence at {df['Date'][j]} - {df['Date'][i - 1]}"
                    )
                j = i
                k = j
        else:
            if k != j:
                if volume[j] < volume[-1] and price[j] > price[-1]:
                    ret.append(f"Bullish divergence at {df['Date'][j]} - now")
                elif volume[j] > volume[-1] and price[j] < price[-1]:
                    ret.append(f"Berrish divergence at {df['Date'][j]} - now")

        return ret

    def calculate_max_deviation_marker(self, price_df, overlap_days=20, excessive=1.5):
        """Calculate the Max_Deviation_Marker for a single symbol using the provided price DataFrame.

        Args:
            symbol (str): The symbol to analyze.
            price_df (pl.DataFrame): Polars DataFrame with price data (Close, Volume, High).
            overlap_days (int): Period for Bollinger Bands and volume MA.
            excessive (float): Threshold multiplier for high volume detection.

        Returns:
            dict: Dictionary with keys 'symbol' and 'max_deviation_marker'.
        """
        # Initialize result
        # Check for required columns
        if not all(col in price_df.columns for col in ["Close", "Volume", "High"]):
            return None

        # Calculate Bollinger Bands and volume MA
        price_df = price_df.with_columns(
            [
                pl.col("Close").rolling_mean(window_size=overlap_days).alias("SMA"),
                pl.col("Close").rolling_std(window_size=overlap_days).alias("STD"),
                (
                    pl.col("Close").rolling_mean(window_size=overlap_days)
                    + pl.col("Close").rolling_std(window_size=overlap_days) * 2
                ).alias("Upper Band"),
                (
                    pl.col("Close").rolling_mean(window_size=overlap_days)
                    - pl.col("Close").rolling_std(window_size=overlap_days) * 2
                ).alias("Lower Band"),
                pl.col("Volume")
                .rolling_mean(window_size=overlap_days)
                .alias("Volume_MA"),
            ]
        )

        # Calculate decission making
        price_df = price_df.with_columns(
            [
                (pl.col("Volume") > pl.col("Volume_MA") * excessive).alias(
                    "High_Volume"
                ),
                (pl.col("Volume") - pl.col("Volume_MA")).alias("Volume_Deviation"),
            ]
        )

        # Find the maximum deviation where High_Volume is True
        filtered_df = price_df.filter(pl.col("High_Volume")).select(
            pl.col("Date").filter(
                pl.col("Volume_Deviation") == pl.col("Volume_Deviation").max()
            )
        )
        if filtered_df.is_empty():
            return None  # or raise a custom exception, e.g., raise ValueError("No high volume data found")
        return filtered_df.item()

    def analyze(self, symbols, number_of_levels, min_peak_distance=0.0, window=3):
        """Run the full volume profile analysis pipeline.

        Args:
            df (pl.DataFrame, optional): Input DataFrame. Uses self.df if None.

        Returns:
            pl.DataFrame: DataFrame with [symbol, shape] and original columns.
        """
        from .core import profile, market, price
        from datetime import datetime

        full_profile_df = self.prepare_volume_profile(
            profile(
                symbols,
                self.resolution,
                self.now,
                self.lookback,
                number_of_levels,
            ),
            number_of_levels,
        )
        market_df = market(symbols).select(["symbol", "price"])
        poc_va_df = self.calculate_poc_and_value_area(full_profile_df)
        shapes_df = self.classify_volume_profile_shape(
            full_profile_df, poc_va_df, min_peak_distance
        )

        # Tích hợp phân tích phân kỳ
        divergence_results = []
        for symbol in symbols:
            df = price(
                symbol,
                self.resolution,
                datetime.fromtimestamp(
                    self.now - self.lookback * 24 * 60 * 60
                ).strftime("%Y-%m-%d"),
                datetime.fromtimestamp(self.now).strftime("%Y-%m-%d"),
            )
            divergences = self.detect_volume_price_divergence(
                df,
                window,
            )
            max_deviation_timestamp = self.calculate_max_deviation_marker(df)

            divergence_results.append(
                {
                    "symbol": symbol,
                    "divergence": divergences[-1] if len(divergences) > 0 else None,
                    "max_deviation_timestamp": max_deviation_timestamp,
                }
            )

        divergence_df = pl.DataFrame(divergence_results)

        return (
            shapes_df.join(
                self.join_with_current_market(
                    full_profile_df,
                    market(symbols).select(["symbol", "price"]),
                ),
                on="symbol",
            )
            .join(
                poc_va_df,
                on="symbol",
            )
            .join(divergence_df, on="symbol", how="left")
            .select(
                [
                    "symbol",
                    "level",
                    "current_price",
                    "vah",
                    "val",
                    "shape",
                    "max_deviation_timestamp",
                    "divergence",
                ]
            )
            .rename({"level": "curent_price_at_level"})
        )

    def plot_heatmap_with_candlestick(
        self, symbol, number_of_levels, overlap_days, excessive=1.1
    ):
        from datetime import datetime, timedelta
        import pandas as pd
        import numpy as np
        import matplotlib.pyplot as plt
        from matplotlib.colors import LinearSegmentedColormap
        import mplfinance as mpf
        from .core import heatmap, profile, price

        # Estimate time range
        from_time = datetime.fromtimestamp(
            self.now - self.lookback * 24 * 60 * 60,
        ).strftime("%Y-%m-%d")
        to_time = datetime.fromtimestamp(self.now).strftime("%Y-%m-%d")

        # Collect data
        candlesticks = price(
            symbol,
            self.resolution,
            from_time,
            to_time,
        ).to_pandas()
        consolidated, levels = heatmap(
            symbol,
            self.resolution,
            self.now,
            self.lookback,
            overlap_days,
            number_of_levels,
        )

        # Convert from_time and to_time to datetime for time axis
        start_date = datetime.strptime(from_time, "%Y-%m-%d")

        # Create time axis for heatmap (starting from the 33rd day to match overlap)
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
        consolidated = np.flipud(
            consolidated
        )  # Flip the consolidated data to match inverted levels

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

        # Create a series for markers (place markers above the high of candles where volume > MA)
        price_df["Marker"] = np.where(
            price_df["High_Volume"], price_df["High"] * 1.01, np.nan
        )

        # Create a series for the max deviation marker
        price_df["Max_Deviation_Marker"] = np.nan
        if pd.notna(max_deviation_idx):
            price_df.loc[max_deviation_idx, "Max_Deviation_Marker"] = (
                price_df.loc[max_deviation_idx, "High"] * 1.02
            )  # Slightly higher for visibility

        # Set up the plot with two subplots
        fig, (ax1, ax2) = plt.subplots(
            2, 1, figsize=(15, 10), gridspec_kw={"height_ratios": [1, 3]}
        )

        # Plot heatmap with imshow
        im = ax1.imshow(
            consolidated,
            aspect="auto",
            interpolation="nearest",
            extent=[0, consolidated.shape[1] - 1, 0, len(levels) - 1],
        )
        ytick_indices = range(0, len(levels), 2)  # Show every 2nd label
        ax1.set_yticks(ytick_indices)
        ax1.set_yticklabels(np.round(levels, 2)[ytick_indices])
        ax1.set_title(
            "Volume Profile Heatmap for {} ({})".format(symbol, self.resolution)
        )
        ax1.set_ylabel("Price Levels")
        ax1.set_xticks(range(0, len(heatmap_dates), max(1, len(heatmap_dates) // 10)))
        ax1.set_xticklabels(
            heatmap_dates[:: max(1, len(heatmap_dates) // 10)],
            rotation=45,
            ha="right",
            fontsize=8,
        )

        # Prepare Bollinger Bands data for plotting
        apds = [
            mpf.make_addplot(
                price_df["SMA"], color="blue", width=1, label="SMA", ax=ax2
            ),
            mpf.make_addplot(
                price_df["Upper Band"], color="red", width=1, label="Upper Band", ax=ax2
            ),
            mpf.make_addplot(
                price_df["Lower Band"],
                color="green",
                width=1,
                label="Lower Band",
                ax=ax2,
            ),
            mpf.make_addplot(
                price_df["Marker"],
                type="scatter",
                marker="^",
                color="red",
                markersize=10,
                label="Max Volume",
                ax=ax2,
            ),
            mpf.make_addplot(
                price_df["Max_Deviation_Marker"],
                type="scatter",
                marker="*",
                color="red",
                markersize=10,
                label="Max Volume Deviation",
                ax=ax2,
            ),
        ]

        # Plot candlestick with Bollinger Bands on the second subplot
        mpf.plot(
            price_df,
            type="candle",
            ax=ax2,
            volume=False,
            style="charles",
            show_nontrading=False,
            addplot=apds,  # Add Bollinger Bands
        )
        ax2.set_title(
            "Candlestick and Volume Chart for {} ({})".format(symbol, self.resolution)
        )
        ax2.set_xlabel("Date")
        ax2.set_ylabel("Price")
        ax2.set_xticks(
            range(0, len(price_dates), max(1, len(price_dates) // 10))
        )  # Show fewer labels if too many
        ax2.set_xticklabels(
            price_dates[:: max(1, len(price_dates) // 10)],
            rotation=45,
            ha="right",
            fontsize=8,
        )

        # Add legend for Bollinger Bands
        ax2.legend()

        # Adjust layout to prevent overlap
        plt.tight_layout()

        # Show plot
        plt.show()
