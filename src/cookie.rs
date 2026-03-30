use std::error::Error;

use chrono::{DateTime, Utc};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone)]
pub struct CloudFlareData {
	pub user_agent: String,
	pub cookies: Vec<String>,
	pub created: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FlareSolverr {
	pub status: String,
	pub message: String,
	pub solution: SolverrSolution,
	pub start_timestamp: i64,
	pub end_timestamp: i64,
	pub version: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverrSolution {
	pub url: String,
	pub status: i32,
	pub cookies: Vec<Cookie>,
	pub user_agent: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cookie {
	domain: String,
	expiry: u64,
	http_only: bool,
	name: String,
	path: String,
	same_site: String,
	secure: bool,
	value: String,
}

impl Cookie {
	fn to_cookie_string(&self) -> String {
		format!("name={}; value={}", self.name, self.value)
	}
}

pub async fn get_cookie(local: &Client) -> Result<CloudFlareData, Box<dyn Error>> {
	let json = json!({
	  "cmd": "request.get",
	  "url": "https://www.fimfiction.net/privacy-policy",
	  "returnOnlyCookies": true,
	  "maxTimeout": 60000
	});
	let res = local
		.post("http://localhost:8191/v1")
		.header("Content-Type", "application/json")
		.body(json.to_string())
		.send()
		.await?
		.json::<FlareSolverr>()
		.await?;
	println!("{}: Cookie message: {}", Utc::now(), res.message);
	let cf_data = CloudFlareData {
		user_agent: res.solution.user_agent,
		cookies: res
			.solution
			.cookies
			.iter()
			.map(|cookie| cookie.to_cookie_string())
			.collect(),
		created: DateTime::from_timestamp_secs(res.end_timestamp).unwrap_or(Utc::now()),
	};
	Ok(cf_data)
}
