//! actix-web routes: the accounting demo UI placeholder and the scanner proxy
//! API stubs.

use actix_web::{get, post, App, HttpResponse, HttpServer, Responder};

use crate::config::Config;

const PLACEHOLDER_HTML: &str = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Railgun accounting demo</title>
  </head>
  <body>
    <h1>Railgun accounting demo</h1>
    <p>Scaffold page. The viewing-key form and the scanner pubkey / init / status
    wiring land with the scanner client thread.</p>
  </body>
</html>
"#;

#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(PLACEHOLDER_HTML)
}

#[get("/api/scanner-pubkey")]
async fn scanner_pubkey() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "publicKey": "" }))
}

#[post("/api/init")]
async fn init() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "walletAddress": "" }))
}

#[get("/api/status")]
async fn status() -> impl Responder {
    HttpResponse::Ok().json(serde_json::json!({ "ready": false, "walletAddress": "" }))
}

pub async fn serve(config: Config) -> anyhow::Result<()> {
    let bind_addr = config.bind_addr.clone();
    HttpServer::new(move || {
        App::new()
            .service(index)
            .service(scanner_pubkey)
            .service(init)
            .service(status)
    })
    .bind(bind_addr)?
    .run()
    .await?;
    Ok(())
}
