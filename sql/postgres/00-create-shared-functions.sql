CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = CURRENT_TIMESTAMP;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION auto_create_monthly_partition()
RETURNS TRIGGER AS $$
DECLARE
    partition_date TEXT;
    partition_name TEXT;
    start_of_month TEXT;
    end_of_month TEXT;
BEGIN
    partition_date := to_char(NEW.created_at, 'YYYYMM');
    partition_name := TG_TABLE_NAME || '_p' || partition_date;

    start_of_month := to_char(NEW.created_at, 'YYYY-MM-01 00:00:00');
    end_of_month := to_char((NEW.created_at + INTERVAL '1 month'), 'YYYY-MM-01 00:00:00');

    IF NOT EXISTS (SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relname = partition_name) THEN
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L);',
            partition_name, TG_TABLE_NAME, start_of_month, end_of_month
        );
        -- Tự động gắn trigger updated_at cho bảng con mới sinh ra
        EXECUTE format(
            'CREATE TRIGGER trg_%I_updated_at BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();',
            partition_name, partition_name
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Hàm tự động sinh Partition theo tháng cho bảng lịch sử giá dựa trên created_at
CREATE OR REPLACE FUNCTION auto_create_ohcl_history_partition()
RETURNS TRIGGER AS $$
DECLARE
    partition_date TEXT;
    partition_name TEXT;
    start_of_month TEXT;
    end_of_month TEXT;
BEGIN
    partition_date := to_char(NEW.created_at, 'YYYYMM');
    partition_name := TG_TABLE_NAME || '_p' || partition_date;

    start_of_month := to_char(NEW.created_at, 'YYYY-MM-01 00:00:00');
    end_of_month := to_char((NEW.created_at + INTERVAL '1 month'), 'YYYY-MM-01 00:00:00');

    IF NOT EXISTS (SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relname = partition_name) THEN
        EXECUTE format(
            'CREATE TABLE IF NOT EXISTS %I PARTITION OF %I FOR VALUES FROM (%L) TO (%L);',
            partition_name, TG_TABLE_NAME, start_of_month, end_of_month
        );
        -- Tự động gắn trigger updated_at cho bảng con mới sinh ra để đồng bộ dữ liệu
        EXECUTE format(
            'CREATE TRIGGER trg_%I_updated_at BEFORE UPDATE ON %I FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();',
            partition_name, partition_name
        );
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
