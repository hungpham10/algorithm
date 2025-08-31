import matplotlib.pyplot as plt
import polars as pl
import numpy as np
import pandas as pd
import mplfinance as mpf


def calculate_atr(df_price, period=14):
    """Calculate Average True Range (ATR) from price DataFrame."""
    # Ensure required columns exist
    if not all(col in df_price.columns for col in ["High", "Low", "Close"]):
        raise ValueError("df_price must contain 'High', 'Low', and 'Close' columns")

    # Calculate True Range components
    df_price = df_price.with_columns(
        [
            (pl.col("High") - pl.col("Low")).alias("high_low"),
            (pl.col("High") - pl.col("Close").shift(1)).abs().alias("high_close"),
            (pl.col("Low") - pl.col("Close").shift(1)).abs().alias("low_close"),
        ]
    )

    # Calculate True Range as the maximum of high_low, high_close, low_close
    true_range = df_price.select(
        pl.max_horizontal(["high_low", "high_close", "low_close"]).alias("true_range")
    )["true_range"]

    # Calculate ATR as the rolling mean of True Range
    return true_range.rolling_mean(window_size=period)


class ClassifyVolumeProfile:
    def __init__(
        self,
        now=None,
        resolution="1D",
        lookback=120,
        value_area_pct=0.7,
        interval_in_hour=24,
    ):
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
        self.interval_in_hour = interval_in_hour

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

    def classify_volume_profile_shape(
        self,
        df_long,
        poc_va_df,
        price_df,
        min_peak_distance=0.0,
        atr_period=14,
        volume_std_multiplier=1.5,
        trend_window=5,
    ):
        """Classify the volume profile shape for each symbol with ATR and statistical analysis.

        Args:
            df_long (pl.DataFrame): Long-format DataFrame with [symbol, price, volume].
            poc_va_df (pl.DataFrame): DataFrame with [symbol, poc_price, poc_volume, vah, val, total_volume].
            price_df (pl.DataFrame): Price DataFrame with [Date, Open, High, Low, Close, Volume, symbol].
            min_peak_distance (float): Minimum distance between volume peaks.
            atr_period (int): Period for ATR calculation.
            volume_std_multiplier (float): Multiplier for volume threshold based on standard deviation.
            trend_window (int): Window to check price trend for I-shape.

        Returns:
            pl.DataFrame: DataFrame with [symbol, shape, peaks, atr].
        """
        from scipy.signal import find_peaks

        def classify_shape(
            group, poc, vah, val, total_volume, price_data, min_peak_distance
        ):
            prices = group["price"]
            volumes = group["volume"]
            profile_high = prices.max()
            profile_low = prices.min()
            price_range = (
                profile_high - profile_low if profile_high > profile_low else 1e-6
            )  # Avoid division by zero

            # Debugging: Print input data stats
            symbol = group["symbol"][0]

            # Calculate ATR for the symbol
            atr = 0.01
            if not price_data.is_empty() and all(
                col in price_data.columns for col in ["High", "Low", "Close"]
            ):
                atr_series = calculate_atr(price_data, atr_period)
                if not atr_series.is_empty() and atr_series.null_count() < len(
                    atr_series
                ):
                    last_atr = atr_series[-1]
                    atr = (
                        last_atr if last_atr is not None else 0.01
                    )  # Check for None (null in Polars)
                else:
                    print(
                        f"Warning: ATR calculation failed for {symbol}, using default ATR=0.01"
                    )

            # Normalize poc_position using ATR
            poc_position = (poc - profile_low) / price_range if price_range > 0 else 0.5
            atr_normalized = atr / price_range if price_range > 0 else 0.01

            # Calculate volume statistics
            volume_mean = volumes.mean()
            volume_std = volumes.std()
            threshold = (
                volume_mean + volume_std * volume_std_multiplier
                if volume_std is not None
                else volume_mean
            )

            # Volume above and below POC
            lower_volume = group.filter(pl.col("price") < poc)["volume"].sum()
            upper_volume = group.filter(pl.col("price") > poc)["volume"].sum()

            # Filter out noise (e.g., volume = 0)
            group = group.filter(pl.col("volume") > 0)
            if group.is_empty():
                print(f"Warning: No valid volume data for {symbol}")
                return "Undefined", [], atr

            # Identify peaks using scipy.signal.find_peaks
            volumes_np = volumes.to_numpy()
            volume_length = len(volumes)
            if volume_length > 0 and price_range > 0:
                distance = max(
                    1, int(min_peak_distance / (price_range / volume_length))
                )  # Ensure distance >= 1
                distance = min(
                    distance, volume_length // 2
                )  # Cap distance to avoid empty peaks
            else:
                distance = 1
                print(
                    f"Warning: Invalid price_range or volume_length for {symbol}, setting distance=1"
                )

            peak_prices, peak_volumes = [], []

            try:
                peaks, _ = find_peaks(volumes_np, distance=distance)

                # Ensure peaks indices are within bounds
                valid_peaks = peaks[peaks < len(prices)]
                if len(valid_peaks) < len(peaks):
                    print(
                        f"Warning: Some peak indices out of bounds for {symbol}, valid_peaks: {len(valid_peaks)}"
                    )

                # Replace Series.take with list indexing
                prices_list = prices.to_list()
                volumes_list = volumes.to_list()
                peak_prices = (
                    [prices_list[i] for i in valid_peaks]
                    if len(valid_peaks) > 0
                    else []
                )
                peak_volumes = (
                    [volumes_list[i] for i in valid_peaks]
                    if len(valid_peaks) > 0
                    else []
                )
            except Exception as e:
                print(f"Error in find_peaks for {symbol}: {e}")

            peaks = [
                {"price": p, "volume": v} for p, v in zip(peak_prices, peak_volumes)
            ]

            # Check price trend for I-shape
            trend_strength = 0
            if (
                not price_data.is_empty()
                and len(price_data) >= trend_window
                and "Close" in price_data.columns
            ):
                recent_prices = price_data["Close"].tail(trend_window)
                price_slope = (recent_prices[-1] - recent_prices[0]) / trend_window
                trend_strength = abs(price_slope) / atr if atr > 0 else 0

            shape = "Undefined"

            # Dynamic thresholds based on ATR
            poc_center_threshold = 0.2 + atr_normalized * 0.5
            poc_skew_threshold = 0.35 + atr_normalized * 0.5
            volume_ratio_threshold = 0.2 - atr_normalized * 0.1

            # Determine shape
            if len(peaks) >= 2:
                max_peak_volume = max([p["volume"] for p in peaks], default=0)
                other_peaks_sum = sum(
                    p["volume"] for p in peaks if p["volume"] != max_peak_volume
                )
                max_peak_price = next(
                    p["price"] for p in peaks if p["volume"] == max_peak_volume
                )

                if max_peak_volume > 1.5 * other_peaks_sum:
                    if (
                        max_peak_price > poc
                        and lower_volume / total_volume < volume_ratio_threshold
                    ):
                        shape = "P-Shaped"
                    elif (
                        max_peak_price < poc
                        and upper_volume / total_volume < volume_ratio_threshold
                    ):
                        shape = "b-Shaped"
                    else:
                        shape = "B-Shaped"
                else:
                    shape = "B-Shaped"

            elif (
                abs(poc_position - 0.5) < poc_center_threshold
                and lower_volume / total_volume > volume_ratio_threshold
                and upper_volume / total_volume > volume_ratio_threshold
            ):
                shape = "D-Shaped"

            elif (
                poc_position > (1 - poc_skew_threshold)
                and lower_volume / total_volume < volume_ratio_threshold
            ):
                shape = "P-Shaped"

            elif (
                poc_position < poc_skew_threshold
                and upper_volume / total_volume < volume_ratio_threshold
            ):
                shape = "b-Shaped"

            elif volumes.max() / total_volume < 0.05 or trend_strength > 1.5:
                shape = "I-Shaped"

            return shape, peaks, atr

        # Initialize results
        shapes = []

        # Iterate over each symbol group
        for symbol, group in df_long.group_by("symbol"):
            poc_data = poc_va_df.filter(pl.col("symbol") == symbol[0])
            if poc_data.is_empty():
                print(f"Warning: No POC data for {symbol[0]}")
                continue
            poc = poc_data["poc_price"][0]
            vah = poc_data["vah"][0]
            val = poc_data["val"][0]
            total_volume = poc_data["total_volume"][0]

            # Get price data for the symbol
            price_data = price_df.filter(pl.col("symbol") == symbol[0])

            # Classify the shape
            shape, peaks, atr = classify_shape(
                group, poc, vah, val, total_volume, price_data, min_peak_distance
            )
            shapes.append(
                {"symbol": symbol[0], "shape": shape, "peaks": peaks, "atr": atr}
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
                    "low" if len(extrema) > 0 and data[-1] < extrema[-1][2] else "high",
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

    def calculate_buy_sell_points(
        self,
        symbol,
        shape,
        current_price,
        vah,
        val,
        poc_price,
        divergence,
        atr,
        price_df,
        trend_window=5,
        use_divergence=False,
    ):
        """Calculate buy and sell price points for a given symbol."""
        # Price range for fallback adjustment
        price_range = (
            vah - val if vah is not None and val is not None and vah > val else 1e-6
        )
        default_adjustment = 0.01 * price_range  # 1% of price range if ATR is too small

        # Use ATR or default adjustment
        atr_adjustment = atr if atr > 0 else default_adjustment
        buy_adjustment = atr_adjustment * 0.5
        sell_adjustment = atr_adjustment * 0.5

        # Divergence adjustments
        is_bullish_divergence = (use_divergence is False) or (
            divergence is not None and "Bullish" in divergence
        )
        is_bearish_divergence = (use_divergence is False) or (
            divergence is not None and "Bearish" in divergence
        )

        # @TODO: improve by calculate potential adjustment for each stock
        if is_bullish_divergence:
            buy_adjustment += (
                atr_adjustment * 0.2
            )  # Lower buy point for bullish divergence
        if is_bearish_divergence:
            sell_adjustment += (
                atr_adjustment * 0.2
            )  # Raise sell point for bearish divergence

        # Trend direction
        trend_strength = 0
        if (
            not price_df.is_empty()
            and len(price_df) >= trend_window
            and "Close" in price_df.columns
        ):
            recent_prices = price_df["Close"].tail(trend_window)
            price_slope = (recent_prices[-1] - recent_prices[0]) / trend_window
            trend_strength = price_slope / atr if atr > 0 else 0

        buy_point = None
        sell_point = None

        if shape == "P-Shaped":
            buy_point = val - buy_adjustment if val is not None else None
            sell_point = (
                vah + sell_adjustment
                if is_bearish_divergence
                else poc_price + sell_adjustment
                if poc_price is not None
                else None
            )

        elif shape == "b-Shaped":
            buy_point = (
                val - buy_adjustment
                if is_bullish_divergence and val is not None
                else None
            )
            sell_point = vah + sell_adjustment if vah is not None else None

        elif shape == "D-Shaped":
            buy_point = val - atr_adjustment * 0.3 if val is not None else None
            sell_point = vah + atr_adjustment * 0.3 if vah is not None else None

        elif shape == "B-Shaped":
            buy_point = (
                val - buy_adjustment
                if is_bullish_divergence and val is not None
                else None
            )
            sell_point = (
                vah + sell_adjustment
                if is_bearish_divergence and vah is not None
                else None
            )

        elif shape == "I-Shaped":
            buy_point = (
                val - buy_adjustment if trend_strength > 0 and val is not None else None
            )
            sell_point = (
                vah + sell_adjustment
                if trend_strength < 0 and vah is not None
                else None
            )

        # Ensure points are positive and reasonable
        if buy_point is not None and buy_point <= 0:
            buy_point = None
        if sell_point is not None and sell_point <= 0:
            sell_point = None

        return {"buy_point": buy_point, "sell_point": sell_point}

    def analyze(
        self,
        symbols,
        number_of_levels,
        min_peak_distance=0.0,
        window=3,
        atr_period=1,
        volume_std_multiplier=1.5,
        trend_window=5,
        use_divergence=False,
    ):
        """Run the full volume profile analysis pipeline with price_df aggregation.

        Args:
            symbols (list): List of stock symbols.
            number_of_levels (int): Number of price levels for volume profile.
            min_peak_distance (float): Minimum distance between volume peaks.
            window (int): Window for divergence detection.
            atr_period (int): Period for ATR calculation.
            volume_std_multiplier (float): Multiplier for volume threshold.
            trend_window (int): Window for price trend analysis.

        Returns:
            pl.DataFrame: DataFrame with [
                    symbol, shape, current_price, vah, val,
                    curent_price_at_level, max_deviation_timestamp, divergence,
                    atr
                ]
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

        # Tích hợp phân tích phân kỳ
        divergence_results = []
        price_dfs = []

        for symbol in symbols:
            df = price(
                symbol,
                self.resolution,
                datetime.fromtimestamp(
                    self.now - self.lookback * 24 * 60 * 60
                ).strftime("%Y-%m-%d"),
                datetime.fromtimestamp(self.now).strftime("%Y-%m-%d"),
            )

            # Add symbol column
            df = df.with_columns(pl.lit(symbol).alias("symbol"))
            price_dfs.append(df)

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

        # Concatenate all price DataFrames into price_df
        price_df = pl.concat(price_dfs, how="vertical")

        # Use improved classify_volume_profile_shape with price_df
        shapes_df = self.classify_volume_profile_shape(
            full_profile_df,
            poc_va_df,
            price_df,
            min_peak_distance,
            atr_period,
            volume_std_multiplier,
            trend_window,
        )

        # Create divergence DataFrame
        divergence_df = pl.DataFrame(divergence_results)

        # Calculate ATR for each symbol
        atr_dict = {}
        for symbol in symbols:
            price_data = price_df.filter(pl.col("symbol") == symbol)
            if not price_data.is_empty() and all(
                col in price_data.columns for col in ["High", "Low", "Close"]
            ):
                atr_series = calculate_atr(price_data, atr_period)
                if not atr_series.is_empty() and atr_series.null_count() < len(
                    atr_series
                ):
                    last_atr = atr_series[-1]
                    atr_dict[symbol] = last_atr if last_atr is not None else 0.01
                else:
                    atr_dict[symbol] = 0.01
            else:
                atr_dict[symbol] = 0.01

        # Join all data
        result_df = (
            shapes_df.join(
                self.join_with_current_market(full_profile_df, market_df),
                on="symbol",
            )
            .join(poc_va_df, on="symbol")
            .join(divergence_df, on="symbol", how="left")
        )

        # Add buy_point and sell_point columns
        result_df = result_df.with_columns(
            pl.struct(
                [
                    "symbol",
                    "shape",
                    "current_price",
                    "vah",
                    "val",
                    "poc_price",
                    "divergence",
                ]
            )
            .map_elements(
                lambda row: self.calculate_buy_sell_points(
                    symbol=row["symbol"],
                    shape=row["shape"],
                    current_price=row["current_price"],
                    vah=row["vah"],
                    val=row["val"],
                    poc_price=row["poc_price"],
                    divergence=row["divergence"],
                    atr=atr_dict.get(row["symbol"], 0.01),
                    price_df=price_df.filter(pl.col("symbol") == row["symbol"]),
                    trend_window=trend_window,
                    use_divergence=use_divergence,
                ),
                return_dtype=pl.Struct(
                    {"buy_point": pl.Float64, "sell_point": pl.Float64}
                ),
            )
            .struct.unnest()
        )

        return result_df.select(
            [
                "symbol",
                "level",
                "current_price",
                "vah",
                "val",
                "max_deviation_timestamp",
                "shape",
                "buy_point",
                "sell_point",
            ]
        ).rename({"level": "current_price_at_level"})

    def plot_heatmap_with_candlestick(
        self,
        symbol,
        number_of_levels,
        overlap_days,
        excessive=1.1,
        top_n=3,
        enable_heatmap=False,
        enable_inverst_ranges=False,
    ):
        from datetime import datetime, timedelta
        from matplotlib.colors import LinearSegmentedColormap
        from .core import heatmap, profile, price

        import pandas as pd
        import seaborn as sns
        import mplfinance as mpf

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
        consolidated, levels, ranges = heatmap(
            symbol,
            self.resolution,
            self.now,
            self.lookback,
            overlap_days,
            number_of_levels,
            self.interval_in_hour,
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

        if enable_heatmap:
            # Set up the plot with two subplots
            fig, (ax1, ax2, ax3) = plt.subplots(
                3, 1, figsize=(15, 10), gridspec_kw={"height_ratios": [1, 3, 1]}
            )
        else:
            # Set up the plot with two subplots
            fig, (ax2, ax3) = plt.subplots(
                2, 1, figsize=(15, 10), gridspec_kw={"height_ratios": [3, 1]}
            )

        if enable_heatmap:
            # Plot heatmap with imshow
            im = ax1.imshow(
                consolidated,
                aspect="auto",
                interpolation="nearest",
                extent=[0, consolidated.shape[1] - 1, 0, len(levels) - 1],
            )
            ytick_indices = range(0, len(levels), 5)  # Show every 2nd label
            ax1.set_yticks(ytick_indices)
            ax1.set_yticklabels(np.round(levels, 5)[ytick_indices])
            ax1.set_title(
                "Volume Profile Heatmap for {} ({})".format(symbol, self.resolution)
            )
            ax1.set_ylabel("Price Levels")
            ax1.set_xticks(
                range(0, len(heatmap_dates), max(1, len(heatmap_dates) // 10))
            )
            ax1.set_xticklabels([])

        # Create a colormap for price range lines
        colors = sns.color_palette("husl", n_colors=top_n)

        # Add horizontal lines for +5% and -5% from the peak
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
                color="green",
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

        if enable_inverst_ranges:
            ranges.reverse()

        # Add price range lines (begin, center, end) with color gradient
        for i, (center, begin, end) in enumerate(ranges):
            if i >= top_n:
                break
            print(levels[begin], levels[center], levels[end])
            color = colors[i % len(colors)]  # Chọn màu từ palette
            apds.extend(
                [
                    mpf.make_addplot(
                        pd.Series(levels[begin], index=price_df.index),
                        color=color,
                        linestyle="--",
                        width=0.5,
                        label=f"Range {i+1} Begin",
                        ax=ax2,
                    ),
                    mpf.make_addplot(
                        pd.Series(levels[center], index=price_df.index),
                        color=color,
                        linestyle="--",
                        width=1.0,
                        label=f"Range {i+1} Center",
                        ax=ax2,
                    ),
                    mpf.make_addplot(
                        pd.Series(levels[end], index=price_df.index),
                        color=color,
                        linestyle="--",
                        width=0.5,
                        label=f"Range {i+1} End",
                        ax=ax2,
                    ),
                ]
            )

        # Plot candlestick with Bollinger Bands and horizontal lines on the second subplot
        mpf.plot(
            price_df,
            type="candle",
            ax=ax2,
            volume=ax3,
            style="charles",
            show_nontrading=False,
            addplot=apds,  # Add Bollinger Bands and horizontal lines
        )
        ax2.set_title(
            "Candlestick and Volume Chart for {} ({})".format(symbol, self.resolution)
        )
        ax2.set_xlabel("Date")
        ax2.set_ylabel("Price")
        ax2.set_xticks(
            range(0, len(price_dates), max(1, len(price_dates) // 10))
        )  # Show fewer labels if too many
        ax2.set_xticklabels([])

        # Add legend for Bollinger Bands and horizontal lines
        ax2.legend()

        # Show plot
        plt.show()
