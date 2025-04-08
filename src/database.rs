use crate::structs::{Blog, CompletionStatus, ContentRating, Story, User};
use crate::utility::{clean_content, parse_date, trim_content};
use chrono::DateTime;
use pony::fimfiction_api::blog::BlogApi;
use pony::fimfiction_api::story::StoryApi;
use pony::fimfiction_api::user::UserData;
use sqlx::{Pool, Postgres};
use std::error::Error;

pub async fn get_blog(id: i32, db: &Pool<Postgres>) -> Result<Option<Blog>, Box<dyn Error>> {
	sqlx::query_as!(
		Blog,
		"SELECT
			id, title, content, link, comments, views,
			author_id, story_id, date_posted, date_cached
		FROM Blogs WHERE id = $1 LIMIT 1;",
		id
	)
	.fetch_optional(db)
	.await
	.map_err(|_| "FixFiction Error: database retrieval error".into())
}

pub async fn insert_blog(
	id: i32, api: &BlogApi<i32>, author_id: i32, story_id: Option<i32>, db: &Pool<Postgres>,
) -> Result<Blog, Box<dyn Error>> {
	sqlx::query_as!(
		Blog,
		"INSERT INTO Blogs 
			(id, title, content, link, comments, views,
			author_id, story_id, date_posted)
		VALUES
			($1, $2, $3, $4, $5, $6, $7, $8, $9)
		ON CONFLICT(id) DO UPDATE SET
			title = EXCLUDED.title,
			content = EXCLUDED.content,
			link = EXCLUDED.link,
			comments = EXCLUDED.comments,
			views = EXCLUDED.views,
			author_id = EXCLUDED.author_id,
			story_id = EXCLUDED.story_id,
			date_posted = EXCLUDED.date_posted,
			date_cached = now()
		RETURNING
			id, title, content, link, comments, views,
			author_id, story_id, date_posted,
			date_cached;",
		id,
		clean_content(api.data.attributes.title.clone()),
		trim_content(api.data.attributes.content.clone(), true),
		api.data.meta.url,
		api.data.attributes.num_comments,
		api.data.attributes.num_views,
		author_id,
		story_id,
		DateTime::parse_from_rfc3339(&api.data.attributes.date_posted)
			.map_err(|_| "FixFiction Error: failed to parse publish date")?
	)
	.fetch_one(db)
	.await
	.map_err(|_| "FixFiction Error: database insertion error".into())
}

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
	.map_err(|_| "FixFiction Error: database retrieval error".into())
}

pub async fn insert_user(
	id: i32, data: &UserData<i32>, image: Option<String>, db: &Pool<Postgres>,
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
		id,
		clean_content(data.attributes.name.clone()),
		clean_content(data.attributes.bio.clone()),
		data.meta.url,
		data.attributes.num_followers,
		data.attributes.num_stories,
		data.attributes.num_blog_posts,
		image,
		data.attributes.color.hex.trim_start_matches("#"),
		DateTime::parse_from_rfc3339(&data.attributes.date_joined)
			.map_err(|_| "FixFiction Error: failed to parse date joined")?
	)
	.fetch_one(db)
	.await
	.map_err(|_| "FixFiction Error: database insertion error".into())
}

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
	.map_err(|_| "FixFiction Error: database retrieval error".into())
}

pub async fn insert_story(
	id: i32, api: StoryApi<i32>, user_id: i32, db: &Pool<Postgres>,
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
		id,
		clean_content(api.data.attributes.title),
		clean_content(api.data.attributes.short_description),
		api.data.attributes.description,
		api.data.attributes.published,
		api.data.meta.url,
		api.data.attributes.cover_image.map(|cover| cover.medium.trim_end_matches("-medium").to_string()),
		api.data.attributes.color.hex.trim_start_matches("#"),
		api.data.attributes.num_views,
		api.data.attributes.total_num_views,
		api.data.attributes.num_words,
		api.data.attributes.num_chapters,
		api.data.attributes.num_comments,
		api.data.attributes.rating,
		CompletionStatus::from(api.data.attributes.completion_status) as _,
		ContentRating::from(api.data.attributes.content_rating) as _,
		api.data.attributes.num_likes,
		api.data.attributes.num_dislikes,
		user_id,
		parse_date(api.data.attributes.date_modified, "modifed")?,
		parse_date(api.data.attributes.date_updated
			.ok_or("Fimfictiion API error: no updated date")?, "updated")?,
		parse_date(api.data.attributes.date_published
			.ok_or("Fimfictiion API error: no publish date")?, "published")?,
	)
	.fetch_one(db)
	.await
	.map_err(|_| "FixFiction Error: database insertion error".into())
}
