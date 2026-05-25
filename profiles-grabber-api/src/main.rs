pub mod discord_api;

use std::{
    cell::OnceCell,
    env,
    sync::{Arc, OnceLock},
};

use axum::{
    Json, extract::{Path, Query, State}, http::{HeaderMap, HeaderValue, StatusCode}, response::{IntoResponse, Response}
};
use reqwest::{Client, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use serde_with::serde_as;
use tokio::net::TcpListener;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

pub struct AppState {
    youtube_reqwest_client: OnceLock<reqwest::Client>,
    discord_reqwest_client: OnceLock<reqwest::Client>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            youtube_reqwest_client: OnceLock::new(),
            discord_reqwest_client: OnceLock::new(),
        }
    }

    pub fn get_discord_client(&self) -> &Client {
        self.discord_reqwest_client.get_or_init(|| {
            let token = env::var("DISCORD_BOT_TOKEN").expect("Token Not Found");
            let auth_str: String = format!("Bot {}", token);

            let mut headers = HeaderMap::new();
            headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_str).unwrap());

            let client = reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .expect("Failed to create client for discord");

            client
        })
    }

    pub fn get_youtube_client(&self) -> &Client {
        self.discord_reqwest_client
            .get_or_init(|| reqwest::Client::new())
    }

    pub fn get_youtube_apikey(&self) -> Option<String> {
        match env::var("YOUTUBE_PUBLIC_APIKEY") {
            Ok(key) => Some(key),
            Err(err) => {
                println!("Unable to init youtube apikey. Error {}", err);
                None
            }
        }
    }
}

#[derive(Debug, Deserialize, IntoParams)]
struct YoutubePlaylistParameters {
    #[allow(dead_code)]
    pub video_amount: Option<i32>,
}

#[serde_as]
#[derive(Deserialize)]
struct YoutubePlaylistQueryResult {
    channel_owner: String,
    video_title: String,
    video_tumbnail_link: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PageInfo {
    pub total_results: i32,
    pub results_per_page: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Default {
    pub url: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Medium {
    pub url: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct High {
    pub url: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Standard {
    pub url: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Maxres {
    pub url: String,
    pub width: i32,
    pub height: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Thumbnails {
    pub default: Default,
    pub medium: Medium,
    pub high: High,
    pub standard: Standard,
    pub maxres: Maxres,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Localized {
    pub title: String,
    pub description: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Snippet {
    pub published_at: String,
    pub channel_id: String,
    pub title: String,
    pub description: String,
    pub thumbnails: Thumbnails,
    pub channel_title: String,
    pub localized: Localized,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub privacy_status: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ContentDetails {
    pub item_count: i32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Items {
    pub kind: String,
    pub etag: String,
    pub id: String,
    pub snippet: Snippet,
    pub status: Status,
    pub content_details: ContentDetails,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Root {
    pub kind: String,
    pub etag: String,
    pub page_info: PageInfo,
    pub items: Vec<Items>,
}

#[utoipa::path(get, path = "/api/youtube/playlist/{id}", params(YoutubePlaylistParameters), responses((status = OK, body=str)))]
async fn get_youtube_playlist_thumbnails(
    Path(id): Path<String>,
    Query(params): Query<YoutubePlaylistParameters>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let yt_client = state.get_youtube_client();

    let api_key = state.get_youtube_apikey();
    if api_key.is_none() {
        return (StatusCode::IM_A_TEAPOT, "Youtube API Key not Configured").into_response();
    };
    let api_key = api_key.unwrap();

    let video_amount = match params.video_amount {
        Some(val) => val,
        None => 3,
    };

    let video_amount = video_amount.clamp(0, 50);

    let url = format!(
        "https://www.googleapis.com/youtube/v3/playlistItems?part=snippet&playlistId={}&key={}&maxResults={}",
        id, api_key, video_amount
    );

    let api_rsp = yt_client.get(url).send().await;
    if let Err(err) = api_rsp {
        return (
            StatusCode::NOT_FOUND,
            format!("Most likely playlist ID not found; Err: {}", err)
        ).into_response()
    };
    let api_rsp = api_rsp.unwrap().text().await.unwrap();

    return api_rsp.into_response();
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let host = "127.0.0.1";
    let port = "8080";

    let shared_app_state = Arc::new(AppState::new());

    let (router, api) = OpenApiRouter::new()
        .routes(routes!(crate::discord_api::get_discord_image))
        .routes(routes!(crate::discord_api::get_discord_user))
        .routes(routes!(get_youtube_playlist_thumbnails))
        .with_state(shared_app_state)
        .split_for_parts();

    let app = router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api));

    let listener = TcpListener::bind(format!("{}:{}", host, port))
        .await
        .unwrap();
    println!("listening on http://{}/{}", host, port);
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
