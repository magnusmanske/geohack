#![forbid(unsafe_code)]
// #[allow(clippy::collapsible_if)]
#![warn(
    clippy::cognitive_complexity,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_link_with_quotes,
    // clippy::doc_markdown,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    // clippy::float_cmp,
    clippy::float_cmp_const,
    clippy::float_equality_without_abs,
    keyword_idents,
    clippy::missing_const_for_fn,
    missing_copy_implementations,
    missing_debug_implementations,
    // clippy::missing_errors_doc,
    clippy::missing_panics_doc,
    clippy::mod_module_files,
    non_ascii_idents,
    noop_method_call,
    // clippy::option_if_let_else,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::semicolon_if_nothing_returned,
    clippy::unseparated_literal_suffix,
    clippy::shadow_unrelated,
    clippy::similar_names,
    clippy::suspicious_operation_groupings,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    clippy::unused_self,
    clippy::use_debug,
    clippy::used_underscore_binding,
    clippy::useless_let_if_seq,
    // clippy::wildcard_dependencies,
    clippy::wildcard_imports
)]
pub mod geo_param;
pub mod geohack;
pub mod map_sources;
pub mod query_parameters;
pub mod server;
pub mod templates;
pub mod traverse_mercator;

use anyhow::Result;
use std::env;
use std::net::Ipv4Addr;

fn ip_string_to_array(ip_str: &str) -> Option<[u8; 4]> {
    match ip_str.parse::<Ipv4Addr>() {
        Ok(ip_addr) => {
            // Ipv4Addr can be converted into [u8; 4]
            // For example, using `into()` or by accessing its octets.
            // A common way is to use `octets()` method.
            Some(ip_addr.octets())
        }
        Err(_) => None, // Return None if parsing fails
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let port: u16 = match env::var("GEOHACK_PORT") {
        Ok(port) => port.as_str().parse::<u16>().unwrap_or(8000),
        Err(_) => 8000,
    };

    let address = match env::var("GEOHACK_ADDRESS") {
        Ok(ip) => ip_string_to_array(&ip),
        Err(_) => None,
    }
    .unwrap_or([0, 0, 0, 0]);

    server::run_server(address, port).await
}
