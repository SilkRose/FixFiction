use crate::fimfiction_api::blog::BlogData;
use crate::fimfiction_api::bookshelf::BookshelfData;
use crate::fimfiction_api::chapter::ChapterData;
use crate::fimfiction_api::group::GroupData;
use crate::fimfiction_api::story::StoryData;
use crate::fimfiction_api::tag::TagData;
use crate::fimfiction_api::thread::ThreadData;
use crate::fimfiction_api::user::UserData;
use crate::structs::{
	Blog, Bookshelf, Chapter, CompletionStatus, ContentRating, Group, Story, Tag, TagLink, TagType,
	Thread, User,
};
use crate::utility::{clean_content, parse_date, trim_content};
use chrono::DateTime;
use sqlx::{Pool, Postgres};
use std::error::Error;

/// Counts the rows for a given table name
///
/// #### Panics
///
/// Panics if the table doesn't exist.
pub async fn count_rows(table: &str, db: &Pool<Postgres>) -> Result<i64, Box<dyn Error>> {
	let query = format!("SELECT count(*) FROM {table}");
	let count: i64 = sqlx::query_scalar(&query).fetch_one(db).await?;
	Ok(count)
}

/// Selects a [Blog] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_blog(id: i32, db: &Pool<Postgres>) -> Result<Option<Blog>, Box<dyn Error>> {
	sqlx::query_as!(
		Blog,
		"SELECT
			id, title, content, link, comments, views,
			author_id, tags, story_id, date_posted, date_cached
		FROM Blogs WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a blog into the database, converting it from [BlogData] to a [Blog]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_blog(
	id: Option<i32>, data: &BlogData<i32>, author_id: i32, story_id: Option<i32>,
	db: &Pool<Postgres>,
) -> Result<Blog, Box<dyn Error>> {
	sqlx::query_as!(
		Blog,
		"INSERT INTO Blogs 
			(id, title, content, link, comments, views,
			author_id, tags, story_id, date_posted)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		ON CONFLICT(id) DO UPDATE SET
			title = EXCLUDED.title,
			content = EXCLUDED.content,
			link = EXCLUDED.link,
			comments = EXCLUDED.comments,
			views = EXCLUDED.views,
			author_id = EXCLUDED.author_id,
			tags = EXCLUDED.tags,
			story_id = EXCLUDED.story_id,
			date_posted = EXCLUDED.date_posted,
			date_cached = now()
		RETURNING
			id, title, content, link, comments, views,
			author_id, tags, story_id, date_posted,
			date_cached;",
		id.unwrap_or(data.id.parse::<i32>()?),
		clean_content(data.attributes.title.clone()),
		trim_content(data.attributes.content.clone(), true),
		data.meta.url,
		data.attributes.num_comments,
		data.attributes.num_views,
		author_id,
		data.attributes.tags.join(", "),
		story_id,
		DateTime::parse_from_rfc3339(&data.attributes.date_posted)
			.map_err(|_| "FixFiction Error: failed to parse date posted")?
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects a [User] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_user(id: i32, db: &Pool<Postgres>) -> Result<Option<User>, Box<dyn Error>> {
	sqlx::query_as!(
		User,
		"SELECT
			id, name, bio, link, followers,
			stories, blogs, profile_pic_url,
			color_hex, date_joined, date_cached
		FROM Authors WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a user into the database, converting it from [UserData] to a [User]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_user(
	id: Option<i32>, data: &UserData<i32>, db: &Pool<Postgres>,
) -> Result<User, Box<dyn Error>> {
	sqlx::query_as!(
		User,
		"INSERT INTO Authors
			(id, name, bio, link, followers, stories,
			blogs, profile_pic_url, color_hex, date_joined)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
		ON CONFLICT(id) DO UPDATE SET
			name = EXCLUDED.name,
			bio = EXCLUDED.bio,
			link = EXCLUDED.link,
			followers = EXCLUDED.followers,
			stories = EXCLUDED.stories,
			blogs = EXCLUDED.blogs,
			profile_pic_url = EXCLUDED.profile_pic_url,
			color_hex = EXCLUDED.color_hex,
			date_joined = EXCLUDED.date_joined,
			date_cached = now()
		RETURNING
			id, name, bio, link, followers,
			stories, blogs, profile_pic_url,
			color_hex, date_joined, date_cached;",
		id.unwrap_or(data.id.parse::<i32>()?),
		clean_content(data.attributes.name.clone()),
		clean_content(data.attributes.bio.clone()),
		data.meta.url,
		data.attributes.num_followers,
		data.attributes.num_stories,
		data.attributes.num_blog_posts,
		(!data.attributes.avatar.r64.ends_with("none_64.png")).then_some(
			data.attributes
				.avatar
				.r256
				.trim_end_matches("-256")
				.to_string(),
		),
		data.attributes.color.hex.trim_start_matches("#"),
		DateTime::parse_from_rfc3339(&data.attributes.date_joined)
			.map_err(|_| "FixFiction Error: failed to parse date joined")?
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects a [Story] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_story(id: i32, db: &Pool<Postgres>) -> Result<Option<Story>, Box<dyn Error>> {
	sqlx::query_as!(
		Story,
		r#"SELECT
			id, title, short_description, description, published, link, cover_url,
			color_hex, views, total_views, words, chapters, comments, rating,
			completion_status AS "completion_status: CompletionStatus",
			content_rating AS "content_rating: ContentRating",
			likes, dislikes, author_id, date_modified,
			date_updated, date_published, date_cached
		FROM Stories WHERE id = $1 LIMIT 1;"#,
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a story into the database, converting it from [StoryData] to a [Story]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_story(
	id: Option<i32>, data: StoryData<i32>, user_id: i32, db: &Pool<Postgres>,
) -> Result<Story, Box<dyn Error>> {
	sqlx::query_as!(
		Story,
		r#"INSERT INTO Stories (
			id, title, short_description, description, published, link, cover_url,
			color_hex, views, total_views, words, chapters, comments, rating,
			completion_status, content_rating, likes, dislikes, author_id,
			date_modified, date_updated, date_published)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22)
		ON CONFLICT(id) DO UPDATE SET
			title = EXCLUDED.title,
			short_description = EXCLUDED.short_description,
			description = EXCLUDED.description,
			published = EXCLUDED.published,
			link = EXCLUDED.link,
			cover_url = EXCLUDED.cover_url,
			color_hex = EXCLUDED.color_hex,
			views = EXCLUDED.views,
			total_views = EXCLUDED.total_views,
			words = EXCLUDED.words,
			chapters = EXCLUDED.chapters,
			comments = EXCLUDED.comments,
			rating = EXCLUDED.rating,
			completion_status = EXCLUDED.completion_status,
			content_rating = EXCLUDED.content_rating,
			likes = EXCLUDED.likes,
			dislikes = EXCLUDED.dislikes,
			author_id = EXCLUDED.author_id,
			date_modified = EXCLUDED.date_modified,
			date_updated = EXCLUDED.date_updated,
			date_published = EXCLUDED.date_published,
			date_cached = now()
		RETURNING 
			id, title, short_description, description, published, link, cover_url,
			color_hex, views, total_views, words, chapters, comments, rating,
			completion_status AS "completion_status: CompletionStatus",
			content_rating AS "content_rating: ContentRating",
			likes, dislikes, author_id, date_modified,
			date_updated, date_published, date_cached;"#,
		id.unwrap_or(data.id.parse::<i32>()?),
		clean_content(data.attributes.title),
		clean_content(data.attributes.short_description),
		data.attributes.description,
		data.attributes.published,
		data.meta.url,
		data.attributes.cover_image.map(|cover| cover.medium.trim_end_matches("-medium").to_string()),
		data.attributes.color.hex.trim_start_matches("#"),
		data.attributes.num_views,
		data.attributes.total_num_views,
		data.attributes.num_words,
		data.attributes.num_chapters,
		data.attributes.num_comments,
		data.attributes.rating,
		CompletionStatus::from(data.attributes.completion_status) as _,
		ContentRating::from(data.attributes.content_rating) as _,
		data.attributes.num_likes,
		data.attributes.num_dislikes,
		user_id,
		parse_date(data.attributes.date_modified, "modified")?,
		parse_date(data.attributes.date_updated
			.ok_or("Fimfictiion API error: no updated date")?, "updated")?,
		parse_date(data.attributes.date_published
			.ok_or("Fimfictiion API error: no publish date")?, "published")?,
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects a [Chapter] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_story_chapter(
	story_id: i32, chapter_num: i32, db: &Pool<Postgres>,
) -> Result<Option<Chapter>, Box<dyn Error>> {
	sqlx::query_as!(
		Chapter,
		r#"SELECT
			id, story_id, chapter_num, title, link, views,
			words, date_published, date_modified, date_cached
		FROM Chapters WHERE story_id = $1 AND chapter_num = $2 LIMIT 1;"#,
		story_id,
		chapter_num
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Selects a [Chapter] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_chapter(id: i32, db: &Pool<Postgres>) -> Result<Option<Chapter>, Box<dyn Error>> {
	sqlx::query_as!(
		Chapter,
		r#"SELECT
			id, story_id, chapter_num, title, link, views,
			words, date_published, date_modified, date_cached
		FROM Chapters WHERE id = $1 LIMIT 1;"#,
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a chapter into the database, converting it from [ChapterData] to a [Chapter]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_chapter(
	id: Option<i32>, data: ChapterData<i32>, story_id: i32, db: &Pool<Postgres>,
) -> Result<Chapter, Box<dyn Error>> {
	sqlx::query_as!(
		Chapter,
		r#"INSERT INTO Chapters 
			(id, story_id, chapter_num, title, link, views,
			words, date_published, date_modified)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9)
		ON CONFLICT(id) DO UPDATE SET
			story_id = EXCLUDED.story_id,
			chapter_num = EXCLUDED.chapter_num,
			title = EXCLUDED.title,
			link = EXCLUDED.link,
			views = EXCLUDED.views,
			words = EXCLUDED.words,
			date_published = EXCLUDED.date_published,
			date_modified = EXCLUDED.date_modified,
			date_cached = now()
		RETURNING
			id, story_id, chapter_num, title, link, views,
			words, date_published, date_modified, date_cached;"#,
		id.unwrap_or(data.id.parse::<i32>()?),
		story_id,
		data.attributes.chapter_number,
		data.attributes.title,
		data.meta.url,
		data.attributes.num_views,
		data.attributes.num_words,
		parse_date(data.attributes.date_published, "published")?,
		parse_date(data.attributes.date_modified, "modified")?,
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects a [Group] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_group(id: i32, db: &Pool<Postgres>) -> Result<Option<Group>, Box<dyn Error>> {
	sqlx::query_as!(
		Group,
		"SELECT
			id, name, description, link, members,
			stories, founder_id, icon_url, nsfw,
			open, hidden, date_created, date_cached
		FROM Groups WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a group into the database, converting it from [GroupData] to a [Group]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_group(
	id: Option<i32>, data: &GroupData<i32>, db: &Pool<Postgres>,
) -> Result<Group, Box<dyn Error>> {
	sqlx::query_as!(
		Group,
		"INSERT INTO Groups
			(id, name, description, link, members,
			stories, founder_id, icon_url, nsfw,
			open, hidden, date_created)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
		ON CONFLICT(id) DO UPDATE SET
			name = EXCLUDED.name,
			description = EXCLUDED.description,
			link = EXCLUDED.link,
			members = EXCLUDED.members,
			stories = EXCLUDED.stories,
			founder_id = EXCLUDED.founder_id,
			icon_url = EXCLUDED.icon_url,
			nsfw = EXCLUDED.nsfw,
			open = EXCLUDED.open,
			hidden = EXCLUDED.hidden,
			date_created = EXCLUDED.date_created,
			date_cached = now()
		RETURNING
			id, name, description, link, members,
			stories, founder_id, icon_url, nsfw,
			open, hidden, date_created, date_cached;",
		id.unwrap_or(data.id.parse::<i32>()?),
		clean_content(data.attributes.name.clone()),
		trim_content(data.attributes.description.clone(), true),
		data.meta.url,
		data.attributes.num_members,
		data.attributes.num_stories,
		data.relationships.founder.data.id.parse::<i32>()?,
		data.attributes
			.icon
			.r512
			.as_ref()
			.map(|icon| icon.trim_end_matches("-512").to_string()),
		data.attributes.nsfw,
		data.attributes.open,
		data.attributes.hidden,
		DateTime::parse_from_rfc3339(&data.attributes.date_created)
			.map_err(|_| "FixFiction Error: failed to parse date created")?
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects a [Bookshelf] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_bookshelf(
	id: i32, db: &Pool<Postgres>,
) -> Result<Option<Bookshelf>, Box<dyn Error>> {
	sqlx::query_as!(
		Bookshelf,
		"SELECT
			id, name, description, link, color, icon_url, stories,
			num_unread, track_unread, quick_add, email_update,
			user_id, order_pos, date_created, date_modified, date_cached
		FROM Bookshelves WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a bookshelf into the database, converting it from [BookshelfData] to a [Bookshelf]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_bookshelf(
	id: Option<i32>, data: &BookshelfData<i32>, user_id: Option<i32>, db: &Pool<Postgres>,
) -> Result<Bookshelf, Box<dyn Error>> {
	sqlx::query_as!(
		Bookshelf,
		"INSERT INTO Bookshelves
			(id, name, description, link, color, icon_url, stories,
			num_unread, track_unread, quick_add, email_update,
			user_id, order_pos, date_created, date_modified)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
		ON CONFLICT(id) DO UPDATE SET
			name = EXCLUDED.name,
			description = EXCLUDED.description,
			link = EXCLUDED.link,
			color = EXCLUDED.color,
			icon_url = EXCLUDED.icon_url,
			stories = EXCLUDED.stories,
			num_unread = EXCLUDED.num_unread,
			track_unread = EXCLUDED.track_unread,
			quick_add = EXCLUDED.quick_add,
			email_update = EXCLUDED.email_update,
			user_id = EXCLUDED.user_id,
			order_pos = EXCLUDED.order_pos,
			date_created = EXCLUDED.date_created,
			date_modified = EXCLUDED.date_modified,
			date_cached = now()
		RETURNING
			id, name, description, link, color, icon_url, stories,
			num_unread, track_unread, quick_add, email_update,
			user_id, order_pos, date_created, date_modified, date_cached;",
		id.unwrap_or(data.id.parse::<i32>()?),
		data.attributes.name,
		data.attributes.description,
		data.meta.url,
		data.attributes.color.trim_start_matches("#"),
		format!("https://raw.githubusercontent.com/SilkRose/Fimfiction-bookshelf-icons/refs/heads/mane/icons/{}/{}/{}.png",
			data.attributes.color.trim_start_matches("#"),
			data.attributes.icon.r#type,
			data.attributes.icon.data.trim_start_matches("&#x")),
		data.attributes.num_stories,
		data.attributes.num_unread,
		data.attributes.track_unread,
		data.attributes.quick_add,
		data.attributes.email_on_update,
		user_id,
		data.attributes.order,
		DateTime::parse_from_rfc3339(&data.attributes.date_created)
			.map_err(|_| "FixFiction Error: failed to parse date created")?,
		DateTime::parse_from_rfc3339(&data.attributes.date_modified)
			.map_err(|_| "FixFiction Error: failed to parse date modified")?
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects a [Thread] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_thread(id: i32, db: &Pool<Postgres>) -> Result<Option<Thread>, Box<dyn Error>> {
	sqlx::query_as!(
		Thread,
		r#"SELECT
			id, group_id, creator_id, last_poster_id, title, link, posts,
			sticky, locked, date_created, date_last_post, date_cached
		FROM Threads WHERE id = $1 LIMIT 1;"#,
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a thread into the database, converting it from [ThreadData] to a [Thread]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_thread(
	id: Option<i32>, data: ThreadData<i32>, group_id: i32, db: &Pool<Postgres>,
) -> Result<Thread, Box<dyn Error>> {
	sqlx::query_as!(
		Thread,
		r#"INSERT INTO Threads 
			(id, group_id, creator_id, last_poster_id, title, link, posts,
			sticky, locked, date_created, date_last_post)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
		ON CONFLICT(id) DO UPDATE SET
			group_id = EXCLUDED.group_id,
			creator_id = EXCLUDED.creator_id,
			last_poster_id = EXCLUDED.last_poster_id,
			title = EXCLUDED.title,
			link = EXCLUDED.link,
			posts = EXCLUDED.posts,
			sticky = EXCLUDED.sticky,
			locked = EXCLUDED.locked,
			date_created = EXCLUDED.date_created,
			date_last_post = EXCLUDED.date_last_post,
			date_cached = now()
		RETURNING
			id, group_id, creator_id, last_poster_id, title, link, posts,
			sticky, locked, date_created, date_last_post, date_cached;"#,
		id.unwrap_or(data.id.parse::<i32>()?),
		group_id,
		data.relationships.creator.data.id.parse::<i32>()?,
		data.relationships.last_poster.data.id.parse::<i32>()?,
		data.attributes.title,
		data.meta.url,
		data.attributes.num_posts,
		data.attributes.sticky,
		data.attributes.locked,
		parse_date(data.attributes.date_created, "published")?,
		parse_date(data.attributes.date_last_post, "modified")?,
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects a [Tag] from the database
///
/// #### Panics
///
/// Panics if it can't select the item from the database.
pub async fn get_tag(id: i32, db: &Pool<Postgres>) -> Result<Option<Tag>, Box<dyn Error>> {
	sqlx::query_as!(
		Tag,
		r#"SELECT
			id, name, type AS "tag_type: TagType", old_id, link, date_cached
		FROM Tags WHERE id = $1 LIMIT 1;"#,
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a tag into the database, converting it from [TagData] to a [Tag]
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_tag(
	id: Option<i32>, tag: TagData<i32>, db: &Pool<Postgres>,
) -> Result<Tag, Box<dyn Error>> {
	sqlx::query_as!(
		Tag,
		r#"INSERT INTO Tags
			(id, name, type, old_id, link)
		VALUES
			($1, $2, $3, $4, $5)
		ON CONFLICT(id) DO UPDATE SET
			name = EXCLUDED.name,
			type = EXCLUDED.type,
			old_id = EXCLUDED.old_id,
			link = EXCLUDED.link,
			date_cached = now()
		RETURNING
			id, name, type AS "tag_type: TagType", old_id, link, date_cached;"#,
		id.unwrap_or(tag.id.parse::<i32>()?),
		tag.attributes.name,
		TagType::from(tag.attributes.r#type) as _,
		tag.meta.old_id,
		tag.meta.url,
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Selects [TagLink]s from the database for a given story ID
///
/// #### Panics
///
/// Panics if it can't select the items from the database.
pub async fn get_tag_links(
	story_id: i32, db: &Pool<Postgres>,
) -> Result<Vec<TagLink>, Box<dyn Error>> {
	sqlx::query_as!(
		TagLink,
		r#"SELECT
			story_id, tag_id, date_cached
		FROM Tag_links WHERE story_id = $1;"#,
		story_id
	)
	.fetch_all(db)
	.await
	.map_err(|e| format!("FixFiction Error: database retrieval error.\n{e}").into())
}

/// Inserts a link between a [Story] and a [Tag] into the database
///
/// #### Panics
///
/// Panics if it can't insert the item into the database.
pub async fn insert_tag_link(
	story_id: i32, tag_id: i32, db: &Pool<Postgres>,
) -> Result<TagLink, Box<dyn Error>> {
	sqlx::query_as!(
		TagLink,
		r#"INSERT INTO Tag_links
			(story_id, tag_id)
		VALUES
			($1, $2)
		ON CONFLICT(story_id, tag_id) DO UPDATE SET
			date_cached = now()
		RETURNING
			story_id, tag_id, date_cached;"#,
		story_id,
		tag_id
	)
	.fetch_one(db)
	.await
	.map_err(|e| format!("FixFiction Error: database insertion error.\n{e}").into())
}

/// Deletes all tag links for a given [Story] ID
///
/// #### Panics
///
/// Panics if it can't delete the items from the database.
pub async fn remove_tag_links(story_id: i32, db: &Pool<Postgres>) -> Result<u64, Box<dyn Error>> {
	let rows = sqlx::query!("DELETE FROM Tag_links WHERE story_id = $1", story_id)
		.execute(db)
		.await
		.map_err(|e| format!("FixFiction Error: database removal error.\n{e}"))?
		.rows_affected();
	Ok(rows)
}
