//! actix-web routes: the accounting demo UI and the scanner proxy API.
//!
//! The app relays ciphertext only — it never sees the plaintext viewing key.
//! The browser fetches the scanner's X25519 public key, seals the pasted
//! viewing key with `seal.js`, and POSTs the ciphertext to `/api/init`. The
//! app forwards it to the scanner over the socket; the scanner opens it,
//! derives the wallet, and enters ready state.

use crate::config::Config;
use crate::listener::Listener;
use crate::scanner::ScannerClient;
use actix_files::Files;
use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use serde::Deserialize;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Shared state for the web handlers.
#[derive(Clone)]
pub struct AppState {
    scanner: ScannerClient,
    ready: Arc<AtomicBool>,
    wallet_address: Arc<Mutex<Option<String>>>,
    listener: Listener,
}

impl AppState {
    fn new(scanner: ScannerClient, listener: Listener) -> Self {
        Self {
            scanner,
            ready: Arc::new(AtomicBool::new(false)),
            wallet_address: Arc::new(Mutex::new(None)),
            listener,
        }
    }
}

/// `POST /api/init` body: the base64 sealed viewing key.
#[derive(Deserialize)]
struct InitRequest {
    #[serde(rename = "keyCiphertext")]
    key_ciphertext: String,
}

/// `GET /` — the UI. `index.html` and `seal.js` are served from `static/`.
#[get("/")]
async fn index() -> impl Responder {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(include_str!("../static/index.html"))
}

/// `GET /seal.js` — the client-side sealed-box bundle.
#[get("/seal.js")]
async fn seal_js() -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/javascript")
        .body(include_str!("../static/seal.js"))
}

/// `GET /api/scanner-pubkey` — proxy the scanner's `pubkey` op.
#[get("/api/scanner-pubkey")]
async fn scanner_pubkey(state: web::Data<AppState>) -> impl Responder {
    match state.scanner.pubkey().await {
        Ok(public_key) => HttpResponse::Ok().json(serde_json::json!({ "publicKey": public_key })),
        Err(err) => HttpResponse::ServiceUnavailable()
            .json(serde_json::json!({ "error": format!("{err:#}") })),
    }
}

/// `POST /api/init` — forward a sealed viewing key to the scanner, start the
/// listener, and report the derived wallet address.
#[post("/api/init")]
async fn init(state: web::Data<AppState>, body: web::Json<InitRequest>) -> impl Responder {
    if state.ready.load(Ordering::SeqCst) {
        return HttpResponse::Conflict()
            .json(serde_json::json!({ "error": "already initialized" }));
    }
    match state.scanner.init(body.key_ciphertext.clone()).await {
        Ok(wallet_address) => {
            state.ready.store(true, Ordering::SeqCst);
            *state.wallet_address.lock().expect("wallet lock") = Some(wallet_address.clone());
            state.listener.start().await;
            HttpResponse::Ok().json(serde_json::json!({ "walletAddress": wallet_address }))
        }
        Err(err) => {
            let message = format!("{err:#}");
            if message.contains("already initialized") {
                // The scanner is already initialized (e.g. the app restarted
                // while the scanner kept its key); reflect that state.
                state.ready.store(true, Ordering::SeqCst);
                HttpResponse::Conflict().json(serde_json::json!({ "error": "already initialized" }))
            } else {
                HttpResponse::BadRequest().json(serde_json::json!({ "error": message }))
            }
        }
    }
}

/// `GET /api/status` — the scanner's ready state and wallet address.
#[get("/api/status")]
async fn status(state: web::Data<AppState>) -> impl Responder {
    let ready = state.ready.load(Ordering::SeqCst);
    let wallet_address = state.wallet_address.lock().expect("wallet lock").clone();
    HttpResponse::Ok().json(serde_json::json!({
        "ready": ready,
        "walletAddress": wallet_address,
    }))
}

/// Build the actix app with the shared state. Exposed for tests.
fn build_app(
    state: AppState,
) -> actix_web::App<
    impl actix_web::dev::ServiceFactory<
        actix_web::dev::ServiceRequest,
        Config = (),
        Response = actix_web::dev::ServiceResponse,
        Error = actix_web::Error,
        InitError = (),
    >,
> {
    let static_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("static");
    App::new()
        .app_data(web::Data::new(state))
        .service(index)
        .service(seal_js)
        .service(scanner_pubkey)
        .service(init)
        .service(status)
        .service(Files::new("/", static_dir).index_file("index.html"))
}

/// Run the actix-web server.
pub async fn serve(
    config: Config,
    scanner: ScannerClient,
    listener: Listener,
) -> anyhow::Result<()> {
    let bind_addr = config.bind_addr.clone();
    let state = AppState::new(scanner, listener);
    HttpServer::new(move || build_app(state.clone()))
        .bind(bind_addr)?
        .run()
        .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Deployment;
    use crate::ledger::Ledger;
    use actix_web::test;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::{UnixListener, UnixStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};
    use std::time::Duration;

    /// A mock scanner speaking the NDJSON wire contract over a Unix socket.
    struct MockScanner {
        socket: PathBuf,
        listener: Option<UnixListener>,
    }

    impl MockScanner {
        fn bind(socket: &Path) -> Self {
            let _ = std::fs::remove_file(socket);
            let listener = UnixListener::bind(socket).expect("bind mock socket");
            Self {
                socket: socket.to_path_buf(),
                listener: Some(listener),
            }
        }

        fn serve(&self, stop: Arc<AtomicBool>) {
            let listener = self
                .listener
                .as_ref()
                .expect("listener")
                .try_clone()
                .expect("clone listener");
            std::thread::spawn(move || {
                while !stop.load(AtomicOrdering::Relaxed) {
                    match listener.accept() {
                        Ok((stream, _)) => serve_connection(stream),
                        Err(_) => break,
                    }
                }
            });
        }
    }

    impl Drop for MockScanner {
        fn drop(&mut self) {
            let _ = std::fs::remove_file(&self.socket);
        }
    }

    fn serve_connection(stream: UnixStream) {
        let mut writer = stream.try_clone().expect("clone stream");
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        loop {
            line.clear();
            let n = reader.read_line(&mut line).expect("read request");
            if n == 0 {
                return;
            }
            let response = mock_response(&line);
            writer
                .write_all(response.as_bytes())
                .expect("write response");
            writer.write_all(b"\n").expect("write newline");
            writer.flush().expect("flush");
        }
    }

    fn mock_response(line: &str) -> String {
        let request: serde_json::Value = serde_json::from_str(line).expect("parse request");
        match request["op"].as_str() {
            Some("pubkey") => r#"{"op":"pubkey","publicKey":"MOCKPUBKEY"}"#.to_string(),
            Some("init") => r#"{"op":"ready","walletAddress":"0xMOCKWALLET"}"#.to_string(),
            Some("ping") => r#"{"op":"pong"}"#.to_string(),
            _ => r#"{"op":"error","message":"unknown op"}"#.to_string(),
        }
    }

    fn temp_socket(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("web-test-{}-{name}.sock", std::process::id()))
    }

    fn test_config() -> Config {
        Config {
            rpc_url: "http://localhost:8545".into(),
            ws_url: "ws://localhost:8545".into(),
            chain_id: 31337,
            scanner_socket: temp_socket("cfg"),
            deployment_path: PathBuf::from("/tmp/deployment.json"),
            ledger_path: PathBuf::from("/tmp/ledger.beancount"),
            bind_addr: "127.0.0.1:0".into(),
            deployment: Deployment {
                proxy: "0xPROXY".into(),
                deployment_block: 1,
            },
        }
    }

    /// Spin up a mock scanner on a real Unix socket plus a client and listener
    /// wired to it. The mock must stay alive for the test, so it is returned.
    async fn test_setup(name: &str) -> (ScannerClient, Listener, MockScanner, Arc<AtomicBool>) {
        let socket = temp_socket(name);
        let mock = MockScanner::bind(&socket);
        let stop = Arc::new(AtomicBool::new(false));
        mock.serve(stop.clone());

        let config = test_config();
        let scanner = crate::scanner::spawn_with(socket, Duration::from_secs(2));
        let listener = Listener::spawn(config, scanner.clone(), Ledger::new());
        (scanner, listener, mock, stop)
    }

    #[actix_web::test]
    async fn index_serves_ui_and_seal_js() {
        let (scanner, listener, _mock, stop) = test_setup("index").await;
        let app = test::init_service(build_app(AppState::new(scanner, listener))).await;

        let req = test::TestRequest::get().uri("/").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let html = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(html.contains("viewing key"), "form missing: {html}");
        assert!(html.contains("seal.js"), "seal.js not referenced: {html}");

        let req = test::TestRequest::get().uri("/seal.js").to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let js = String::from_utf8(body.to_vec()).expect("utf8");
        assert!(
            js.contains("railgun-scanner-vk") && js.contains("keyCiphertext"),
            "seal.js content missing: {js}"
        );

        stop.store(true, AtomicOrdering::Relaxed);
    }

    #[actix_web::test]
    async fn scanner_pubkey_proxies_the_scanner() {
        let (scanner, listener, _mock, stop) = test_setup("pubkey").await;
        let app = test::init_service(build_app(AppState::new(scanner, listener))).await;

        let req = test::TestRequest::get()
            .uri("/api/scanner-pubkey")
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["publicKey"], "MOCKPUBKEY");

        stop.store(true, AtomicOrdering::Relaxed);
    }

    #[actix_web::test]
    async fn init_returns_wallet_starts_listener_and_status_goes_ready() {
        let (scanner, listener, _mock, stop) = test_setup("init").await;
        let app = test::init_service(build_app(AppState::new(scanner, listener))).await;

        let req = test::TestRequest::get().uri("/api/status").to_request();
        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["ready"], false);
        assert!(json["walletAddress"].is_null());

        let req = test::TestRequest::post()
            .uri("/api/init")
            .set_json(serde_json::json!({ "keyCiphertext": "c2VhbGVk" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());
        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["walletAddress"], "0xMOCKWALLET");

        let req = test::TestRequest::get().uri("/api/status").to_request();
        let resp = test::call_service(&app, req).await;
        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["ready"], true);
        assert_eq!(json["walletAddress"], "0xMOCKWALLET");

        stop.store(true, AtomicOrdering::Relaxed);
    }

    #[actix_web::test]
    async fn second_init_returns_a_clear_error() {
        let (scanner, listener, _mock, stop) = test_setup("second-init").await;
        let app = test::init_service(build_app(AppState::new(scanner, listener))).await;

        let req = test::TestRequest::post()
            .uri("/api/init")
            .set_json(serde_json::json!({ "keyCiphertext": "c2VhbGVk" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert!(resp.status().is_success());

        let req = test::TestRequest::post()
            .uri("/api/init")
            .set_json(serde_json::json!({ "keyCiphertext": "c2VhbGVk" }))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), actix_web::http::StatusCode::CONFLICT);
        let body = test::read_body(resp).await;
        let json: serde_json::Value = serde_json::from_slice(&body).expect("json");
        assert_eq!(json["error"], "already initialized");

        stop.store(true, AtomicOrdering::Relaxed);
    }
}
