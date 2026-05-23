use tokio::net::TcpListener;
use utoipa_axum::{
    routes,
    PathItemExt,
    router::OpenApiRouter
};
use utoipa_swagger_ui::SwaggerUi;

#[utoipa::path(get, path = "", responses((status = OK, body=str)))]
async fn hello_world() -> &'static str {
    return "Hello There!";
}

#[tokio::main]
async fn main() {
    let (router, api) = OpenApiRouter::new()
        .routes(routes!(hello_world))
        .split_for_parts();

    let app = router.merge(SwaggerUi::new("/docs").url("/api-docs/openapi.json", api));

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
