CREATE TABLE IF NOT EXISTS Fimfic_status (
	datetime       timestamptz NOT NULL PRIMARY KEY,
	api_duration   integer,
	round_trip     integer
);
