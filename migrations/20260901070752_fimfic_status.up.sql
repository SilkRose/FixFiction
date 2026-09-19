CREATE TABLE IF NOT EXISTS Fimfic_status_minutes (
	datetime       timestamptz NOT NULL PRIMARY KEY,
	api_duration   integer,
	round_trip     integer,
	challenged     boolean     NOT NULL
);

CREATE TABLE IF NOT EXISTS Fimfic_status_days (
	date                 date    NOT NULL PRIMARY KEY,
	minutes_total        integer NOT NULL,
	minutes_checked      integer NOT NULL,
	minutes_offline      integer NOT NULL,
	minutes_challenged   integer NOT NULL,
	incidents_offline    integer NOT NULL,
	incidents_challenged integer NOT NULL,
	average_api_duration float   NOT NULL,
	average_round_trip   float   NOT NULL
);
