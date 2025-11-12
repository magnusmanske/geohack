#[allow(clippy::collapsible_if)]
pub mod geo_param;
pub mod geohack;
pub mod map_sources;
pub mod traverse_mercator;

use anyhow::Result;
use axum::Router;
use std::{env, net::SocketAddr};
use tower_http::{compression::CompressionLayer, services::ServeDir, trace::TraceLayer};

// /// Web server handler functions
// pub mod handlers {
//     use super::*;
//     use std::collections::HashMap;

//     pub async fn handle_request(params: HashMap<String, String>) -> Result<String, String> {
//         let mut geohack = GeoHack::new();

//         // Initialize from request
//         geohack.init_from_request(&params)?;

//         // Fetch template (would be async in production)
//         let template_content = fetch_template(&geohack.lang, &geohack.globe).await?;

//         // Set page content
//         geohack.set_page_content(&template_content);

//         // Process and return final output
//         Ok(geohack.process())
//     }

//     async fn fetch_template(lang: &str, globe: &str) -> Result<String, String> {
//         // This would fetch from Wikipedia API in production
//         // For now, returning empty template
//         Ok(String::new())
//     }
// }

async fn run_server() -> Result<()> {
    tracing_subscriber::fmt::init();

    // let cors = CorsLayer::new().allow_origin(Any);

    let app = Router::new()
        // .route("/", get(root))
        // .route("/supported_properties", get(supported_properties))
        // .route("/item/{prop}/{id}", get(item))
        // .route("/meta_item/{prop}/{id}", get(meta_item))
        // .route("/graph/{prop}/{id}", get(graph))
        // .route("/extend/{item}", get(extend))
        // .route("/merge", get(merge_info).post(merge))
        .nest_service("/images", ServeDir::new("images"))
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new());
    //        .layer(cors);

    let port: u16 = match env::var("GEOHACK_PORT") {
        Ok(port) => port.as_str().parse::<u16>().unwrap_or(8000),
        Err(_) => 8000,
    };

    let address = [0, 0, 0, 0]; // TODOO env::var("AC2WD_ADDRESS")

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
