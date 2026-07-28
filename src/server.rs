use crate::{geohack::GeoHack, query_parameters::QueryParameters, templates::Templates};
use anyhow::Result;
use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{AppendHeaders, Html, IntoResponse, Response},
    routing::get,
};
use std::net::SocketAddr;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

#[derive(Debug, Clone, Default)]
struct AppState {
    templates: Templates,
}

#[axum::debug_handler]
async fn main_css() -> Response {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, "text/css".parse().unwrap());
    let css_content = include_str!("../data/main.css").to_string();
    (StatusCode::OK, headers, css_content).into_response()
    // TODO: check if the short form below works
    // (
    //     AppendHeaders([(CONTENT_TYPE, "text/css")]),
    //     include_str!("../data/main.css").to_string(),
    // )
}

#[axum::debug_handler]
async fn favicon_ico() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/favicon.ico");
    (AppendHeaders([(CONTENT_TYPE, "image/x-icon")]), FAVICON)
}

#[axum::debug_handler]
async fn siteicon_png() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/siteicon.png");
    (AppendHeaders([(CONTENT_TYPE, "image/png")]), FAVICON)
}

#[axum::debug_handler]
async fn external_png() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/external.png");
    (AppendHeaders([(CONTENT_TYPE, "image/png")]), FAVICON)
}

#[axum::debug_handler]
async fn bullet_gif() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/bullet.gif");
    (AppendHeaders([(CONTENT_TYPE, "image/gif")]), FAVICON)
}

#[axum::debug_handler]
async fn lock_icon_gif() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/lock_icon.gif");
    (AppendHeaders([(CONTENT_TYPE, "image/gif")]), FAVICON)
}

#[axum::debug_handler]
async fn index() -> Html<String> {
    let html = include_str!("../data/index.html").to_string();
    Html(html)
}

#[axum::debug_handler]
async fn testcases_html() -> Html<String> {
    let html = include_str!("../data/testcases.html").to_string();
    Html(html)
}

#[axum::debug_handler]
async fn geohack(
    State(state): State<AppState>,
    headers: HeaderMap,
    params: Query<QueryParameters>,
) -> Result<Html<String>, (StatusCode, String)> {
    let mut query = params.0;
    query.set_http_referrer(
        headers
            .get("referer")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string()),
    );
    let mut geohack = GeoHack::new().map_err(|error| {
        tracing::error!(%error, "GeoHack::new failed");
        internal_server_error()
    })?;
    // Failures here are caused by invalid user input (bad params etc.)
    geohack.init_from_query(&query).map_err(|error| {
        tracing::info!(%error, params = query.params(), "Rejected query parameters");
        (StatusCode::BAD_REQUEST, format!("Bad request: {error}"))
    })?;

    let language = geohack.lang().trim().to_ascii_lowercase();
    let globe = geohack.globe().trim().to_ascii_lowercase();
    let purge = query.purge();
    let template_content = state
        .templates
        .load(&language, &globe, &query, purge)
        .await
        .map_err(|error| {
            tracing::error!(%error, language, globe, "Failed to load GeoTemplate");
            internal_server_error()
        })?;

    geohack.set_page_content(&template_content);
    let html = geohack
        .process()
        .map_err(|error| {
            tracing::error!(%error, "Failed to process template");
            internal_server_error()
        })?
        .replace("</html>", "<!-- Rust code --></html>");

    Ok(Html(html))
}

fn internal_server_error() -> (StatusCode, String) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}

pub async fn run_server(address: [u8; 4], port: u16) -> Result<()> {
    tracing_subscriber::fmt::init();

    let state = AppState::default();

    // let cors = CorsLayer::new().allow_origin(Any);

    let app = Router::new()
        .route("/", get(index))
        .route("/index.php", get(index))
        .route("/main.css", get(main_css))
        .route("/geohack.php", get(geohack))
        .route("/favicon.ico", get(favicon_ico))
        .route("/geohack/siteicon.png", get(siteicon_png))
        .route("/siteicon.png", get(siteicon_png))
        .route("/bullet.gif", get(bullet_gif))
        .route("/lock_icon.gif", get(lock_icon_gif))
        .route("/external.png", get(external_png))
        .route("/testcases.html", get(testcases_html))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        //        .layer(cors),
        .with_state(state);

    let ip_addr = std::net::Ipv4Addr::from(address);
    tracing::info!("Starting server on http://{ip_addr}:{port}");

    let addr = SocketAddr::from((address, port));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
