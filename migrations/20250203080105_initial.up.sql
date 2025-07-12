CREATE TYPE completion_status AS enum (
	'incomplete',
	'complete',
	'hiatus',
	'cancelled'
);

CREATE TYPE content_rating AS enum (
	'everyone',
	'teen',
	'mature'
);

CREATE TABLE IF NOT EXISTS Authors (
	id              integer     NOT NULL PRIMARY KEY,
	name            text        NOT NULL,
	bio             text        NOT NULL,
	link            text        NOT NULL,
	followers       integer     NOT NULL,
	stories         integer     NOT NULL,
	blogs           integer     NOT NULL,
	profile_pic_url text        NULL,
	color_hex       char(6)     NOT NULL,
	date_joined     timestamptz NOT NULL,
	date_cached     timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS Stories (
	id                integer           NOT NULL PRIMARY KEY,
	title             text              NOT NULL,
	short_description text              NOT NULL,
	description       text              NOT NULL,
	published         boolean           NOT NULL,
	link              text              NOT NULL,
	cover_url         text              NULL,
	color_hex         char(6)           NOT NULL,
	views             integer           NOT NULL,
	total_views       integer           NOT NULL,
	words             integer           NOT NULL,
	chapters          integer           NOT NULL,
	comments          integer           NOT NULL,
	rating            integer           NOT NULL,
	completion_status completion_status NOT NULL,
	content_rating    content_rating    NOT NULL,
	tags              text              NOT NULL,
	likes             integer           NOT NULL,
	dislikes          integer           NOT NULL,
	author_id         integer           NOT NULL,
	date_modified     timestamptz       NOT NULL,
	date_updated      timestamptz       NOT NULL,
	date_published    timestamptz       NOT NULL,
	date_cached       timestamptz       NOT NULL DEFAULT now(),

	CONSTRAINT stories_author_id_fk FOREIGN KEY (author_id)
		REFERENCES Authors (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Chapters (
	id               integer          NOT NULL PRIMARY KEY,
	story_id         integer          NOT NULL,
	chapter_num      integer          NOT NULL,
	title            text             NOT NULL,
	link             text             NOT NULL,
	views            integer          NOT NULL,
	words            integer          NOT NULL,
	date_published   timestamptz      NOT NULL,
	date_modified    timestamptz      NOT NULL,
	date_cached      timestamptz      NOT NULL DEFAULT now(),

	CONSTRAINT chapter_story_id_fk FOREIGN KEY (story_id)
		REFERENCES Stories (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Blogs (
	id          integer     NOT NULL PRIMARY KEY,
	title       text        NOT NULL,
	content     text        NOT NULL,
	link        text        NOT NULL,
	comments    integer     NOT NULL,
	views       integer     NOT NULL,
	author_id   integer     NOT NULL,
	tags        text        NOT NULL,
	story_id    integer     NULL,
	date_posted timestamptz NOT NULL,
	date_cached timestamptz NOT NULL DEFAULT now(),

	CONSTRAINT blogs_author_id_fk FOREIGN KEY (author_id)
		REFERENCES Authors (id) ON DELETE CASCADE,

	CONSTRAINT blogs_story_id_fk FOREIGN KEY (story_id)
		REFERENCES Stories (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Groups (
	id           integer     NOT NULL PRIMARY KEY,
	name         text        NOT NULL,
	description  text        NOT NULL,
	link         text        NOT NULL,
	members      integer     NOT NULL,
	stories      integer     NOT NULL,
	founder_id   integer     NOT NULL,
	nsfw         boolean     NOT NULL,
	open         boolean     NOT NULL,
	hidden       boolean     NOT NULL,
	icon_url     text        NULL,
	date_created timestamptz NOT NULL,
	date_cached  timestamptz NOT NULL DEFAULT now(),

	CONSTRAINT groups_founder_id_fk FOREIGN KEY (founder_id)
		REFERENCES Authors (id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS Bookshelves (
	id            integer     NOT NULL PRIMARY KEY,
	name          text        NOT NULL,
	description   text        NOT NULL,
	link          text        NOT NULL,
	color         text        NOT NULL,
	icon_url      text        NOT NULL,
	stories       integer     NOT NULL,
	num_unread    integer     NULL,
	track_unread  boolean     NOT NULL,
	quick_add     boolean     NOT NULL,
	email_update  boolean     NOT NULL,
	user_id       integer     NULL,
	order_pos     integer     NOT NULL,
	date_created  timestamptz NOT NULL,
	date_modified timestamptz NOT NULL,
	date_cached   timestamptz NOT NULL DEFAULT now(),

	CONSTRAINT bookshelves_user_id_fk FOREIGN KEY (user_id)
		REFERENCES Authors (id) ON DELETE CASCADE
);
