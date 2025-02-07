create type completion_status as enum (
	'incomplete',
	'complete',
	'hiatus',
	'cancelled'
);

create type content_rating as enum (
	'everyone',
	'teen',
	'mature'
);

create table authors (
	id integer not null primary key,
	name text not null,
	bio text not null,
	followers integer not null,
	stories integer not null,
	blogs integer not null,
	profile_pic_256 text,
	color_hex char(6) not null,
	date_cached timestamptz not null default current_timestamp,
);

create table stories (
	id integer not null primary key,
	title text not null,
	short_description text not null,
	cover_medium_url text,
	color_hex char(6) not null,
	views integer not null,
	words: integer not null.
	chapters: integer not null,
	comments integer not null,
	completion_status completion_status not null,
	content_rating content_rating not null,
	likes integer not null,
	dislikes integer not null,
	author_id integer not null,
	date_cached timestamptz not null default current_timestamp,

	constraint stories_author_id_fk foreign key (author_id)
		references authors (id)
);

create table chapters (
	story_id integer not null,
	chapter_num integer not null,
	title text not null,
	content text not null,
	views integer not null,
	words integer not null,
	date_cached timestamptz not null default current_timestamp,

	constraint chapter_story_id_fk foreign key (story_id)
		references stories (id),

	constraint chapters_pk primary key (story_id, chapter_num)
);

create table blogs (
	id integer not null primary key,
	title text not null,
	content text not null,
	comments integer not null,
	views integer not null,
	author_id integer not null,
	story_id integer,
	date_cached timestamptz not null default current_timestamp,

	constraint blogs_author_id_fk foreign key (author_id)
		references authors (id),

	constraint blogs_story_id_fk foreign key (story_id)
		references stories (id)
);
