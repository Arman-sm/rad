// This crate is dedicated to adapters for casting audio using the network.

mod utils;

mod udp;
mod simple_http;

pub use udp::init_udp_adapter;
pub use simple_http::init_simple_http_adapter;