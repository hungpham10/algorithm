import matplotlib.pyplot as plt
import polars as pl
import numpy as np
import pandas as pd
import mplfinance as mpf


class ClassifyVolumeProfile:
    def __init__(self, resolution="1D", lookback=120, value_area_pct=0.7):
        self.resolution = resolution
        self.lookback = lookback
        self.value_area_pct = value_area_pct

    def prepare_volume_profile(self, df, number_of_levels):
        """Transform DataFrame into long format with price and volume per level.

        Args:
            df (pl.DataFrame, optional): Input DataFrame. Uses self.df if None.

        Returns:
            pl.DataFrame: Long-format DataFrame with columns [symbol, price, volume].
        """
        if df is None:
            raise ValueError(
                "DataFrame must be provided either at initialization or as argument."
            )

        level_columns = [f"level_{i}" for i in range(number_of_levels)]

        # Calculate price step for each symbol
        df = df.with_columns(
            price_step=(pl.col("price_at_level_last") - pl.col("price_at_level_first"))
            / (number_of_levels - 1)
        )

        # Create a list of price levels for each symbol
        df = df.with_columns(
            prices=pl.struct(["price_at_level_first", "price_step"]).map_elements(
                lambda x: [
                    x["price_at_level_first"] + i * x["price_step"]
                    for i in range(number_of_levels)
                ],
                return_dtype=pl.List(pl.Float64),
            )
        )

        # Melt the DataFrame to long format
        df_long = df.melt(
            id_vars=["symbol", "prices", "price_at_level_first", "price_at_level_last"],
            value_vars=level_columns,
            variable_name="level",
            value_name="volume",
        )

        # Extract the price for each level
        df_long = df_long.with_columns(
            price=pl.col("prices").list.get(
                pl.col("level").str.extract(r"level_(\d+)").cast(pl.Int32)
            )
        )

        return df_long.select(["symbol", "price", "volume"])

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
                        return "P-Shaped"
                    if max_peak_price < poc and upper_volume / total_volume < 0.2:
                        return "b-Shaped"
                return "B-Shaped"

            # Existing logic for other shapes
            if (
                abs(poc_position - 0.5) < 0.2
                and lower_volume / total_volume > 0.2
                and upper_volume / total_volume > 0.2
            ):
                return "D-Shaped"
            if poc_position > 0.65 and lower_volume / total_volume < 0.2:
                return "P-Shaped"
            if poc_position < 0.35 and upper_volume / total_volume < 0.2:
                return "b-Shaped"
            if group["volume"].max() / total_volume < 0.05:
                return "I-Shaped"
            return "Undefined"

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
            shape = classify_shape(
                group, poc, vah, val, total_volume, min_peak_distance
            )
            shapes.append({"symbol": symbol[0], "shape": shape})

        # Convert results to DataFrame
        shapes_df = pl.DataFrame(shapes)
        return shapes_df

    def plot_heatmap_with_candlestick(self, symbol, number_of_levels, overlap_days):
        from time import time as now
        from datetime import datetime, timedelta
        from matplotlib.colors import LinearSegmentedColormap
        from .core import heatmap, profile, price

        # Estimate time range
        from_time = datetime.fromtimestamp(
            int(now()) - self.lookback * 24 * 60 * 60,
        ).strftime("%Y-%m-%d")
        to_time = datetime.fromtimestamp(int(now())).strftime("%Y-%m-%d")

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
        ax1.set_xticks(
            range(0, len(heatmap_dates), max(1, len(heatmap_dates) // 10))
        )  # Show fewer labels if too many
        ax1.set_xticklabels(
            heatmap_dates[:: max(1, len(heatmap_dates) // 10)],
            rotation=45,
            ha="right",
            fontsize=8,
        )

        # Plot candlestick with volume on the second subplot
        mpf.plot(
            price_df,
            type="candle",
            ax=ax2,
            volume=False,
            style="charles",
            show_nontrading=False,
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

        # Adjust layout to prevent overlap
        plt.tight_layout()

        # Show plot
        plt.show()

    def analyze(self, symbols, number_of_levels, min_peak_distance=0.0):
        """Run the full volume profile analysis pipeline.

        Args:
            df (pl.DataFrame, optional): Input DataFrame. Uses self.df if None.

        Returns:
            pl.DataFrame: DataFrame with [symbol, shape] and original columns.
        """
        from .core import profile, price

        full_profile_df = self.prepare_volume_profile(
            profile(
                symbols,
                self.resolution,
                self.lookback,
                number_of_levels,
            ),
            number_of_levels,
        )
        poc_va_df = self.calculate_poc_and_value_area(full_profile_df)
        shapes_df = self.classify_volume_profile_shape(
            full_profile_df, poc_va_df, min_peak_distance
        )
        return shapes_df
