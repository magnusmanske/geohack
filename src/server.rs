use crate::{geohack::GeoHack, query_parameters::QueryParameters, templates::Templates};
use anyhow::Result;
use axum::{
    Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::CONTENT_TYPE},
    response::{Html, IntoResponse},
    routing::get,
};
use std::net::SocketAddr;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

#[derive(Debug, Clone, Default)]
struct AppState {
    templates: Templates,
}

// Note: the `[(CONTENT_TYPE, ...)]` form replaces the `Content-Type` that axum
// derives from the body type. `AppendHeaders` would add a second value instead,
// yielding a malformed header such as `application/octet-stream,image/png`.

#[axum::debug_handler]
async fn main_css() -> impl IntoResponse {
    (
        [(CONTENT_TYPE, "text/css; charset=utf-8")],
        include_str!("../data/main.css"),
    )
}

#[axum::debug_handler]
async fn favicon_ico() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/favicon.ico");
    ([(CONTENT_TYPE, "image/x-icon")], FAVICON)
}

#[axum::debug_handler]
async fn siteicon_png() -> impl IntoResponse {
    const SITEICON: &[u8] = include_bytes!("../data/siteicon.png");
    ([(CONTENT_TYPE, "image/png")], SITEICON)
}

#[axum::debug_handler]
async fn external_png() -> impl IntoResponse {
    const EXTERNAL: &[u8] = include_bytes!("../data/external.png");
    ([(CONTENT_TYPE, "image/png")], EXTERNAL)
}

#[axum::debug_handler]
async fn bullet_gif() -> impl IntoResponse {
    const BULLET: &[u8] = include_bytes!("../data/bullet.gif");
    ([(CONTENT_TYPE, "image/gif")], BULLET)
}

#[axum::debug_handler]
async fn lock_icon_gif() -> impl IntoResponse {
    const LOCK_ICON: &[u8] = include_bytes!("../data/lock_icon.gif");
    ([(CONTENT_TYPE, "image/gif")], LOCK_ICON)
}

#[axum::debug_handler]
async fn index() -> Html<&'static str> {
    Html(include_str!("../data/index.html"))
}

#[axum::debug_handler]
async fn testcases_html() -> Html<&'static str> {
    Html(include_str!("../data/testcases.html"))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn content_types(response: &axum::response::Response) -> Vec<String> {
        response
            .headers()
            .get_all(CONTENT_TYPE)
            .iter()
            .map(|value| value.to_str().unwrap_or_default().to_string())
            .collect()
    }

    /// Each static asset must send exactly one `Content-Type`; appending to the
    /// body-derived default used to produce `application/octet-stream,image/png`.
    #[tokio::test]
    async fn test_static_assets_have_a_single_content_type() {
        let assets = [
            (main_css().await.into_response(), "text/css; charset=utf-8"),
            (favicon_ico().await.into_response(), "image/x-icon"),
            (siteicon_png().await.into_response(), "image/png"),
            (external_png().await.into_response(), "image/png"),
            (bullet_gif().await.into_response(), "image/gif"),
            (lock_icon_gif().await.into_response(), "image/gif"),
        ];
        for (response, expected) in assets {
            assert_eq!(content_types(&response), vec![expected.to_string()]);
        }
    }

    #[tokio::test]
    async fn test_html_pages_declare_utf8() {
        for response in [
            index().await.into_response(),
            testcases_html().await.into_response(),
        ] {
            assert_eq!(
                content_types(&response),
                vec!["text/html; charset=utf-8".to_string()]
            );
        }
    }
}
