CREATE TABLE IF NOT EXISTS  public.tbl_fireant_mention (
	id         SERIAL      PRIMARY KEY,
	symbol     VARCHAR(20) NOT NULL,
	mention    INT4 NULL,
	positive   INT4 NULL,
	negative   INT4 NULL,
	created_at TIMESTAMPTZ DEFAULT CURRENT_TIMESTAMP NOT NULL
);

CREATE TABLE IF NOT EXISTS public.tbl_tcbs_orders (
	id 	   SERIAL      PRIMARY KEY,
	symbol     VARCHAR(5)  NOT NULL,
	side       INT2        NOT NULL,
	price      FLOAT       NOT NULL,
	volume     INT4        NOT NULL,
	ordered_at TIMESTAMPTZ NOT NULL
);

CREATE TABLE IF NOT EXISTS  public.tbl_crons (
	id       SERIAL       PRIMARY KEY,
	interval VARCHAR(100) NOT NULL,
	resolver TEXT         NOT NULL	
);

