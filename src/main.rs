use actix_web::error::ErrorBadRequest;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use clap::Arg;
use http::Uri;
use mediaproxy_common::query::Query;
use serde::Deserialize;

mod forwarder;

async fn router(query: web::Json<Query>, uri: web::Data<Uri>) -> actix_web::Result<HttpResponse> {
    forwarder::forward(query.into_inner(), uri.get_ref().clone())
        .await?
        .await
}

#[derive(Deserialize)]
pub struct FingerprintRequest {
    fingerprint: String,
}

async fn fingerprint(
    web::Query(info): web::Query<FingerprintRequest>,
    uri: web::Data<Uri>,
) -> actix_web::Result<HttpResponse> {
    match Query::from_fingerprint(info.fingerprint) {
        Ok(query) => {
            forwarder::forward(query, uri.get_ref().clone())
                .await?
                .await
        }
        Err(error) => Err(ErrorBadRequest(error)),
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    let matches = clap::App::new("MediaProxy Router")
        .arg(
            Arg::with_name("forward_addr")
                .long("forward")
                .takes_value(true)
                .value_name("FWD ADDR")
                .required(true),
        )
        .arg(
            Arg::with_name("listen_addr")
                .long("listen")
                .takes_value(true)
                .value_name("LISTEN ADDR")
                .required(false)
                .default_value("127.0.0.1:8080"),
        )
        .get_matches();

    let listen_addr = matches.value_of("listen_addr").unwrap();
    let forward_addr = matches.value_of("forward_addr").unwrap();
    let forward_uri = Uri::builder()
        .scheme("http")
        .authority(forward_addr)
        .path_and_query("")
        .build()
        .unwrap();

    println!("Binding {}", listen_addr);
    println!("Forwarding requests to {}", forward_addr);

    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(web::JsonConfig::default().limit(4096))
            .data(forward_uri.clone())
            .service(
                web::resource("/")
                    .route(web::post().to(router))
                    .route(web::get().to(fingerprint)),
            )
    })
    .bind(listen_addr)?
    .run()
    .await
}
