use crate::database::insert_tag;
use crate::fimfiction_api::tag::TagApi;
use crate::structs::AppState;
use crate::utility::parse_fimfic_response;
use std::error::Error;

pub async fn request_tags(app: &AppState) -> Result<(), Box<dyn Error>> {
	let fimfic = "https://www.fimfiction.net/api/v2/story-tags?fields%5Bstory_tag%5D=name,description,type,num_stories";
	let api = parse_fimfic_response::<TagApi<i32>>(&app.api, fimfic).await?;
	for tag in api.data {
		insert_tag(None, tag, &app.db).await?;
	}
	Ok(())
}
