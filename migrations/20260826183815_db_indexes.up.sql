CREATE TYPE archive_status AS enum (
	'un_archived',
	'partially_archived',
	'archived'
);

CREATE TYPE fimfic_status AS enum (
	'public',
	'deleted'
);

CREATE TABLE IF NOT EXISTS Author_index (
	id             integer        NOT NULL PRIMARY KEY,
	archive_status archive_status NOT NULL,
	fimfic_status  fimfic_status  NOT NULL,
	date_checked   timestamptz    NOT NULL DEFAULT now()
);

ALTER TABLE Authors
	ADD CONSTRAINT author_index_fk FOREIGN KEY (id)
		REFERENCES Author_index (id) ON DELETE CASCADE;

CREATE TABLE IF NOT EXISTS Story_index (
	id             integer        NOT NULL PRIMARY KEY,
	archive_status archive_status NOT NULL,
	fimfic_status  fimfic_status  NOT NULL,
	date_checked   timestamptz    NOT NULL DEFAULT now()
);

ALTER TABLE Stories
	ADD CONSTRAINT story_index_fk FOREIGN KEY (id)
		REFERENCES Story_index (id) ON DELETE CASCADE;

CREATE TABLE IF NOT EXISTS Blog_index (
	id             integer        NOT NULL PRIMARY KEY,
	archive_status archive_status NOT NULL,
	fimfic_status  fimfic_status  NOT NULL,
	date_checked   timestamptz    NOT NULL DEFAULT now()
);

ALTER TABLE Blogs
	ADD CONSTRAINT blog_index_fk FOREIGN KEY (id)
		REFERENCES Blog_index (id) ON DELETE CASCADE;

CREATE TABLE IF NOT EXISTS group_index (
	id             integer        NOT NULL PRIMARY KEY,
	archive_status archive_status NOT NULL,
	fimfic_status  fimfic_status  NOT NULL,
	date_checked   timestamptz    NOT NULL DEFAULT now()
);

ALTER TABLE Groups
	ADD CONSTRAINT group_index_fk FOREIGN KEY (id)
		REFERENCES Group_index (id) ON DELETE CASCADE;
