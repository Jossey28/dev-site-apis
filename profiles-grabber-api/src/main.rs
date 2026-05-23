use std::env;

use anyhow::Ok;
use axum::{extract::Path, http::response};
use reqwest::header::AUTHORIZATION;
use tokio::net::TcpListener;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr};

// #[derive(Clone, Debug)]
// struct AppState {
//     reqwest_client: reqwest::Client,
// }

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscordUserObject {
    pub id: String,
    pub username: String,
    pub avatar: String,
    pub discriminator: String,
    pub public_flags: i64,
    pub flags: i64,
    pub banner: String,
    pub accent_color: Option<serde_json::Value>,
    pub global_name: String,
    pub avatar_decoration_data: Option<serde_json::Value>,
    pub collectibles: Option<serde_json::Value>,
    pub display_name_styles: DisplayNameStyles,
    pub banner_color: Option<serde_json::Value>,
    pub clan: Clan,
    pub primary_guild: Clan,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clan {
    pub identity_guild_id: String,
    pub identity_enabled: bool,
    pub tag: String,
    pub badge: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DisplayNameStyles {
    pub font_id: i64,
    pub effect_id: i64,
    pub colors: Vec<i64>,
}

#[utoipa::path(get, path = "/api/discord/{id}/user", responses((status = OK, body=str)))]
async fn get_discord_user(Path(id): Path<u64>) {
    /// let mut headers = HeaderMap::new();
    // headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer my-token"));

    // let client = reqwest::Client::builder()
    //     .default_headers(headers)
    //     .build()?;


    let client = reqwest::Client::new();
    let token = env::var("DISCORD_BOT_TOKEN").expect("Token Not Found");
    let url = format!("https://discord.com/api/v10/users/{}", id);

    let response = client
        .get(url)
        .header(AUTHORIZATION, format!("Bot {}", token))
        .send().await.expect("Failed to send request");

    println!("{:#?}", response.text().await.expect("Couldn't get text"));
}



#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let (router, api) = OpenApiRouter::new()
        .routes(routes!(get_discord_user))
        .split_for_parts();

    let app = router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api));

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
