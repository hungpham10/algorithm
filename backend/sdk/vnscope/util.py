import polars as pl


def align_and_concat(dfs, column_order=None):
    # Lấy tất cả các cột duy nhất từ các DataFrame
    all_columns = set().union(*(df.columns for df in dfs))

    # Nếu không chỉ định column_order, sắp xếp theo bảng chữ cái
    if column_order is None:
        column_order = sorted(all_columns)
    else:
        # Đảm bảo column_order bao gồm tất cả các cột
        missing_cols = all_columns - set(column_order)
        if missing_cols:
            raise ValueError(f"Column order missing: {missing_cols}")

    # Chuẩn hóa mỗi DataFrame: thêm cột thiếu và sắp xếp cột
    aligned_dfs = []
    for df in dfs:
        # Thêm cột thiếu với giá trị null
        for col in all_columns:
            if col not in df.columns:
                df = df.with_columns(pl.lit(None).alias(col))
        # Sắp xếp cột theo column_order
        df = df.select(column_order)
        aligned_dfs.append(df)

    # Nối các DataFrame
    return pl.concat(aligned_dfs, how="vertical")


def group_files_by_symbol(file_list, date, table):
    # Tạo biểu thức chính quy để trích xuất symbol
    pattern = rf"^{date}/{table}-(\w+)-[0-9]+\.parquet$"

    # Sử dụng defaultdict để nhóm tệp theo symbol
    files_by_symbol = defaultdict(list)

    for file_path in file_list:
        match = re.match(pattern, file_path)
        if match:
            symbol = match.group(1)  # Lấy symbol từ nhóm đầu tiên
            files_by_symbol[symbol].append(file_path)

    return dict(files_by_symbol)
