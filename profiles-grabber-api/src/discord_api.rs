use std::{fmt::Display, sync::Arc};

use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header::CONTENT_TYPE},
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use serde_with::{DisplayFromStr, serde_as};
use utoipa::ToSchema;

use crate::AppState;

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

// Begin route definitions; TODO! I don't like how this file is structed, look to see if anyone else does it better.

#[utoipa::path(get, path = "/api/discord/{id}/user", responses((status = OK, body=DiscordUserObject)))]
pub async fn get_discord_user(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Json<DiscordUserObject> {
    let client = state.create_discord_client();
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
pub async fn get_discord_image(
    Path(id): Path<u64>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let user = get_discord_user(Path(id), State(Arc::clone(&state))).await;
    let avatar = user.avatar.clone();

    let client = state.create_discord_client();

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
