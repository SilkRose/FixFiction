CREATE TYPE tag_type AS enum (
	'character',
	'genre',
	'rating',
	'content',
	'series',
	'warning',
	'universe'
);

ALTER TABLE Stories DROP COLUMN tags;
DELETE FROM Stories;

CREATE TABLE IF NOT EXISTS Tags (
	id          integer     NOT NULL PRIMARY KEY,
	name        text        NOT NULL,
	type        tag_type    NOT NULL,
	old_id      text        NULL,
	link        text        NOT NULL,
	date_cached timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS Tag_links (
	story_id    integer NOT NULL,
	tag_id      integer NOT NULL,
	date_cached timestamptz NOT NULL DEFAULT now(),

	CONSTRAINT tag_links_story_id_fk FOREIGN KEY (story_id)
		REFERENCES Stories (id) ON DELETE CASCADE,

	CONSTRAINT tag_links_tag_id_fk FOREIGN KEY (tag_id)
		REFERENCES Tags (id) ON DELETE CASCADE,

	CONSTRAINT tag_links_pk PRIMARY KEY (story_id, tag_id)
);
