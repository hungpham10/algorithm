CREATE TABLE IF NOT EXISTS  public.tbl_processes (
	id        SERIAL       PRIMARY KEY,
	instance  VARCHAR(100) NOT NULL,
    command   VARCHAR(100) NOT NULL,
    arguments VARCHAR(300) NOT NULL
);
