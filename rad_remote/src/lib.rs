// This crate is dedicated to exposing a REST interface for controlling the state of the program.

use std::{net::SocketAddr, sync::{Arc, Mutex}};

use actix_web::{middleware::NormalizePath, web, App, HttpServer};
use rad_compositor::{adapter::AdapterHandle, cmp_reg::CompositionRegistry};

mod cmp;

struct State {
    cmp_reg: Arc<Mutex<CompositionRegistry<1024>>>,
    #[allow(dead_code)]
    adapters: Mutex<Vec<AdapterHandle>>
}

// TODO: Add authentication
// TODO: Add ability to control and monitor the adapters.
/// Starts the REST API used to control and configure the service.
pub async fn start_remote_server(cmp_reg: Arc<Mutex<CompositionRegistry<1024>>>, adapters: Vec<AdapterHandle>, addr: SocketAddr) -> std::io::Result<()> { 
    let state = State {
        cmp_reg: cmp_reg,
        adapters: Mutex::new(adapters)
    };

    let data = web::Data::new(state);

    HttpServer::new(move || {
        App::new()
            .app_data(data.clone())
            .wrap(NormalizePath::trim())
            .service(
                web::scope("/v1/cmp")
                    .service(cmp::get_info)
                    .service(cmp::get_info_json)
                    .service(cmp::set_time)
                    .service(cmp::set_pause)
                    .service(cmp::upload)
            )
    })
    .workers(2)
    .bind(addr)
    .unwrap()
    .run()
    .await
}