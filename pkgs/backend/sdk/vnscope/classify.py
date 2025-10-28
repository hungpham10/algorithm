import matplotlib.pyplot as plt
import numpy as np
import pandas as pd
import mplfinance as mpf
import seaborn as sns

from .symbols import Symbols


class ClassifyVolumeProfile:
    def __init__(
        self,
        now=None,
        resolution="1D",
        lookback=120,
        interval_in_hour=24,
    ):
        from datetime import datetime, timezone, timedelta

        self.symbols = Symbols()

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
        consolidated, levels, ranges = self.symbols.heatmap(
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

        # Create time axis for heatmap (starting from the overlap_days to match overlap)
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

        # Add price range lines (begin, center, end) with a single shared label
        # per range to reduce legend items
        for i, (center, begin, end) in enumerate(ranges):
            if i >= top_n:
                break
            color = colors[i % len(colors)]  # Chọn màu từ palette
            range_label = f"Range {i+1}"  # Shared label for the entire range
            apds.extend(
                [
                    mpf.make_addplot(
                        pd.Series(levels[begin], index=price_df.index),
                        color=color,
                        linestyle="--",
                        width=0.5,
                        # Only label the first one to avoid duplicates
                        label=range_label if i == 0 else None,
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
            figsize=(40, 24),  # Increased figure size
            tight_layout=True,  # Improve overall spacing
            legend_loc="upper left",  # Position legend to avoid overlap
            legend_fontsize=8,  # Smaller font for legend to fit better
            returnfig=False,
        )

        # Note: For full integration with custom subplots, consider using
        # mpf.plot with returnfig=True and manual subplot addition. This version
        # plots heatmap separately if enabled, and candlestick in its own figure
