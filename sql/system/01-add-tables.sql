CREATE TABLE IF NOT EXISTS  public.tbl_fireant_mention (
	id serial4 NOT NULL,
	symbol varchar NOT NULL,
	mention int4 NULL,
	positive int4 NULL,
	negative int4 NULL,
	created_at timestamptz DEFAULT CURRENT_TIMESTAMP NOT NULL,
	CONSTRAINT newtable_pk PRIMARY KEY (id)
);
