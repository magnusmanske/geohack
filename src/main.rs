#![forbid(unsafe_code)]
#[allow(clippy::collapsible_if)]
// #![warn(
//     clippy::cognitive_complexity,
//     clippy::dbg_macro,
//     clippy::debug_assert_with_mut_call,
//     clippy::doc_link_with_quotes,
//     clippy::doc_markdown,
//     clippy::empty_line_after_outer_attr,
//     // clippy::empty_structs_with_brackets,
//     clippy::float_cmp,
//     clippy::float_cmp_const,
//     clippy::float_equality_without_abs,
//     keyword_idents,
//     // clippy::missing_const_for_fn,
//     missing_copy_implementations,
//     missing_debug_implementations,
//     // clippy::missing_errors_doc,
//     clippy::missing_panics_doc,
//     clippy::mod_module_files,
//     non_ascii_idents,
//     noop_method_call,
//     // clippy::option_if_let_else,
//     // clippy::print_stderr,
//     // clippy::print_stdout,
//     clippy::semicolon_if_nothing_returned,
//     clippy::unseparated_literal_suffix,
//     clippy::shadow_unrelated,
//     clippy::similar_names,
//     clippy::suspicious_operation_groupings,
//     unused_crate_dependencies,
//     unused_extern_crates,
//     unused_import_braces,
//     clippy::unused_self,
//     // clippy::use_debug,
//     clippy::used_underscore_binding,
//     clippy::useless_let_if_seq,
//     // clippy::wildcard_dependencies,
//     // clippy::wildcard_imports
// )]
pub mod geo_param;
pub mod geohack;
pub mod geohack_parameters;
pub mod map_sources;
pub mod templates;
pub mod traverse_mercator;

use crate::{geohack::GeoHack, geohack_parameters::GehohackParameters, templates::Templates};
use anyhow::Result;
use axum::{
    Router,
    extract::{Query, State},
    http::HeaderMap,
    response::{AppendHeaders, Html, IntoResponse},
    routing::get,
};
use reqwest::StatusCode;
use std::{env, net::SocketAddr};
use tower_http::{compression::CompressionLayer, trace::TraceLayer};

#[derive(Debug, Clone, Default)]
struct AppState {
    templates: Templates,
}

#[axum::debug_handler]
async fn main_css() -> impl IntoResponse {
    (
        AppendHeaders([(reqwest::header::CONTENT_TYPE, "text/css")]),
        include_str!("../data/main.css").to_string(),
    )
}

#[axum::debug_handler]
async fn favicon_ico() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/favicon.ico");
    (
        AppendHeaders([(reqwest::header::CONTENT_TYPE, "image/x-icon")]),
        FAVICON,
    )
}

#[axum::debug_handler]
async fn external_png() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/external.png");
    (
        AppendHeaders([(reqwest::header::CONTENT_TYPE, "image/png")]),
        FAVICON,
    )
}

#[axum::debug_handler]
async fn bullet_gif() -> impl IntoResponse {
    const FAVICON: &[u8] = include_bytes!("../data/bullet.gif");
    (
        AppendHeaders([(reqwest::header::CONTENT_TYPE, "image/gif")]),
        FAVICON,
    )
}

#[axum::debug_handler]
async fn geohack(
    State(state): State<AppState>,
    headers: HeaderMap,
    params: Query<GehohackParameters>,
) -> Result<Html<String>, StatusCode> {
    let mut query = params.0;
    query.http_referrer = headers
        .get("referer")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let mut geohack = GeoHack::new().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    geohack
        .init_from_query(query.clone())
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let language = geohack.lang.trim().to_ascii_lowercase();
    let globe = geohack.globe.trim().to_ascii_lowercase();
    let template_content = state
        .templates
        .load(&language, &globe, &query)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    geohack.set_page_content(&template_content);
    let html = geohack.process();

    Ok(Html(html))
}

async fn run_server() -> Result<()> {
    tracing_subscriber::fmt::init();

    let state = AppState::default();

    // let cors = CorsLayer::new().allow_origin(Any);

    let app = Router::new()
        // .route("/", get(root))
        .route("/main.css", get(main_css))
        .route("/geohack.php", get(geohack))
        .route("/favicon.ico", get(favicon_ico))
        .route("/geohack/siteicon.png", get(favicon_ico))
        .route("/bullet.gif", get(bullet_gif))
        .route("/external.png", get(external_png))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        //        .layer(cors),
        .with_state(state);

    let port: u16 = match env::var("GEOHACK_PORT") {
        Ok(port) => port.as_str().parse::<u16>().unwrap_or(8000),
        Err(_) => 8000,
    };

    let address = [0, 0, 0, 0]; // TODOO env::var("AC2WD_ADDRESS")
    println!("Starting server on http://localhost:{}", port);

    let addr = SocketAddr::from((address, port));
    tracing::debug!("listening on {}", addr);
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("Could not create listener");
    axum::serve(listener, app)
        .await
        .expect("Could not start server");

    Ok(())
}
#[tokio::main]
async fn main() -> Result<()> {
    run_server().await
}

/*
TESTING:

curl -sg 'https://geohack.toolforge.org/geohack.php?pagename=G%C3%B6ttingen&params=51_32_02_N_09_56_08_E_type:city(118946)_region:DE-NI' > g1.html

curl -sg 'http://localhost:8000/geohack.php?pagename=G%C3%B6ttingen&params=51_32_02_N_09_56_08_E_type:city(118946)_region:DE-NI' > g2.html
 ; diff -b g1.html g2.html

 */
