CREATE TABLE IF NOT EXISTS Threads (
	id             integer     NOT NULL PRIMARY KEY,
	group_id       integer     NOT NULL,
	creator_id     integer     NOT NULL,
	last_poster_id integer     NOT NULL,
	title          text        NOT NULL,
	link           text        NOT NULL,
	posts          integer     NOT NULL,
	sticky         boolean     NOT NULL,
	locked         boolean     NOT NULL,
	date_created   timestamptz NOT NULL,
	date_last_post timestamptz NOT NULL,
	date_cached    timestamptz NOT NULL DEFAULT now(),

	CONSTRAINT threads_group_id_fk FOREIGN KEY (group_id)
		REFERENCES Groups (id) ON DELETE CASCADE,
	
	CONSTRAINT threads_creator_id_fk FOREIGN KEY (creator_id)
		REFERENCES Authors (id) ON DELETE CASCADE,
	
	CONSTRAINT threads_last_poster_id_fk FOREIGN KEY (last_poster_id)
		REFERENCES Authors (id) ON DELETE CASCADE
);
