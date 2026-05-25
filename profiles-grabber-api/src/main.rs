pub mod discord_api;

use std::{env, sync::Arc};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};
use reqwest::{Client, header::AUTHORIZATION};
use tokio::net::TcpListener;
use utoipa::IntoParams;
use utoipa_axum::{router::OpenApiRouter, routes};
use utoipa_swagger_ui::SwaggerUi;

pub struct AppState {
    #[allow(dead_code)]
    reqwest_client: Option<reqwest::Client>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            reqwest_client: None,
        }
    }

    pub fn create_discord_client(&self) {
        let token = env::var("DISCORD_BOT_TOKEN").expect("Token Not Found");
        let auth_str: String = format!("Bot {}", token);

        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_str(&auth_str).unwrap());

        let client = reqwest::Client::builder()
            .default_headers(headers)
            .build()
            .expect("Failed to create client for discord");

        client
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

#[derive(Debug, serde::Deserialize,IntoParams)]
struct YoutubePlaylistParameters {
    #[allow(dead_code)]
    pub video_amount: Option<i32>,
}

#[utoipa::path(get, path = "/api/youtube/playlist/{id}", params(YoutubePlaylistParameters), responses((status = OK, body=str)))]
async fn get_youtube_playlist(
    Path(id): Path<String>,
    Query(params): Query<YoutubePlaylistParameters>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let api_key = state.get_youtube_apikey();
    if let None = api_key {
        return (StatusCode::IM_A_TEAPOT, "Youtube API Key not Configured").into_response();
    };

    let api_key = api_key.unwrap();

    let video_amount = match params.video_amount {
        Some(val) => val,
        None => 3,
    };

    let video_amount = video_amount.clamp(0, 50);

    let url = format!(
        "https://www.googleapis.com/youtube/v3/playlists?part=snippet&id={}&key={}&maxResults={}",
        id, api_key, video_amount
    );

    let api_response = Client::new();


    return "hi".into_response();
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
        .routes(routes!(get_youtube_playlist))
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
