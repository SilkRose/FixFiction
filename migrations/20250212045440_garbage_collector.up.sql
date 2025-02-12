CREATE OR REPLACE PROCEDURE garbage_collector(tables text[], min_time timestamptz)
LANGUAGE plpgsql
AS $$
DECLARE
	table_name text;
BEGIN
	FOREACH table_name IN ARRAY tables
	LOOP
		EXECUTE format('DELETE FROM %I WHERE date_cached < $1', lower(table_name))
		USING min_time;
	END LOOP;
END;
$$;

CREATE OR REPLACE FUNCTION count_rows(tables text[])
RETURNS integer[]
LANGUAGE plpgsql
AS $$
DECLARE
	table_name text;
	row_count integer;
	counts integer[] := ARRAY[]::integer[];
BEGIN
	FOREACH table_name IN ARRAY tables
	LOOP
		EXECUTE format('SELECT count(*) FROM %I', lower(table_name)) INTO row_count;
		counts := array_append(counts, row_count);
	END LOOP;
	RETURN counts;
END;
$$;
