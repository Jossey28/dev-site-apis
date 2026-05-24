use std::{env, fmt::Display, sync::Arc};

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use tokio::net::TcpListener;
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone, Debug)]
struct AppState {
    reqwest_client: reqwest::Client,
}

impl AppState {
    fn new() -> Self {
        let token = env::var("DISCORD_BOT_TOKEN").expect("Token Not Found");
        let auth_str = format!("Bot {}", token);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_str).unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to create client");

        Self {
            reqwest_client: client,
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Snowflake(#[serde_as(as = "DisplayFromStr")] pub u64);
impl Display for Snowflake {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DiscordUserObject {
    // Group the important information up here
    pub id: Snowflake,
    pub username: String,
    pub discriminator: String,
    pub global_name: Option<String>,
    pub avatar: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub primary_guild: Option<Option<UserPrimaryGuild>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub premium_type: Option<PremiumType>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "u8")]
pub enum PremiumType {
    None = 0,
    NitroClassic = 1,
    Nitro = 2,
    NitroBasic = 3,
}

impl TryFrom<u8> for PremiumType {
    // https://doc.rust-lang.org/rust-by-example/conversion/try_from_try_into.html
    type Error = String;
    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::NitroClassic),
            2 => Ok(Self::Nitro),
            3 => Ok(Self::NitroBasic),
            _ => Err(format!("Unknown PremiumType: {}", value)),
        }
    }
}

#[serde_as]
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPrimaryGuild {
    pub identity_guild_id: Option<Snowflake>,
    pub identity_enabled: Option<bool>,
    pub tag: Option<String>,
    pub badge: Option<String>,
}

#[utoipa::path(get, path = "/api/discord/{id}/user", responses((status = OK, body=DiscordUserObject)))]
async fn get_discord_user(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Json<DiscordUserObject> {
    let client = &state.reqwest_client;
    let url = format!("https://discord.com/api/v10/users/{}", id);

    let response = client
        .get(url)
        .send()
        .await
        .expect("Failed to send request")
        .json::<DiscordUserObject>()
        .await
        .expect("Failed to convert to json");

    Json(response)
}

#[utoipa::path(get, path = "/api/discord/{id}/image", responses((status = OK, body=str)))]
async fn get_discord_image(Path(id): Path<u64>, State(state): State<Arc<AppState>>) -> Response {
    let user = get_discord_user(Path(id), State(state.clone())).await;
    let avatar = user.avatar.clone();

    let client = &state.reqwest_client;

    if avatar.is_none() {
        let index: u64 = match user.discriminator == "0" {
            true => (user.id.clone().0 >> 22) % 6,
            false => user.discriminator.parse().expect("Unable to parse string "),
        };

        let url = format!("https://cdn.discordapp.com/embed/avatars/{}.png", index);

        let response = client.get(&url).send().await.unwrap();
        let image_data = response.bytes().await.unwrap();
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "image/png")
            .body(Body::from(image_data))
            .unwrap()
            .into_response()
    } else {
        let avatar = avatar.unwrap();

        let url = format!(
            "https://cdn.discordapp.com/avatars/{}/{}.png",
            user.id, avatar
        );
        let response = client.get(&url).send().await.unwrap();
        let image_data = response.bytes().await.unwrap();
        Response::builder()
            .status(StatusCode::OK)
            .header(CONTENT_TYPE, "image/png")
            .body(Body::from(image_data))
            .unwrap()
            .into_response()
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let shared_app_state = Arc::new(AppState::new());

    let (router, api) = OpenApiRouter::new()
        .routes(routes!(get_discord_user))
        .routes(routes!(get_discord_image))
        .with_state(shared_app_state)
        .split_for_parts();

    let app = router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api));

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
