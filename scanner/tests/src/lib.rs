//! Helpers for the cross-language protocol tests: locate the scanner, build it
//! if needed, spawn it on a private Unix socket, and load the test-bed fixture.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Once;
use std::time::{Duration, Instant};

use base64::Engine;
use serde::Deserialize;

static COUNTER: AtomicU64 = AtomicU64::new(0);
static BUILD: Once = Once::new();

pub fn scanner_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("tests crate has a parent")
        .to_path_buf()
}

pub fn repo_root() -> PathBuf {
    scanner_dir()
        .parent()
        .expect("scanner dir has a parent")
        .to_path_buf()
}

fn ensure_built() {
    BUILD.call_once(|| {
        let status = Command::new("npm")
            .args(["run", "build"])
            .current_dir(scanner_dir())
            .status()
            .expect("run npm build");
        assert!(status.success(), "npm run build failed");
    });
}

/// A spawned scanner process bound to a private socket. Killed on drop.
pub struct ScannerProcess {
    child: Child,
    socket: PathBuf,
}

impl ScannerProcess {
    pub fn spawn() -> Self {
        Self::spawn_with_key_service("local")
    }

    pub fn spawn_with_key_service(key_service: &str) -> Self {
        ensure_built();
        let run_dir = scanner_dir().join(".run");
        fs::create_dir_all(&run_dir).expect("create .run");
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let socket = run_dir.join(format!("protocol-{}-{n}.sock", std::process::id()));
        let _ = fs::remove_file(&socket);

        let child = Command::new("node")
            .arg("dist/index.js")
            .current_dir(scanner_dir())
            .env("SCANNER_SOCKET", &socket)
            .env("KEY_SERVICE", key_service)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn scanner");

        let deadline = Instant::now() + Duration::from_secs(20);
        while !socket.exists() {
            assert!(
                Instant::now() < deadline,
                "scanner did not bind socket {}",
                socket.display()
            );
            std::thread::sleep(Duration::from_millis(20));
        }

        Self { child, socket }
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

impl Drop for ScannerProcess {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = fs::remove_file(&self.socket);
    }
}

#[derive(Debug, Deserialize)]
pub struct Fixture {
    pub blocks: Vec<railgun_scanner_handle::BlockLogs>,
}

#[derive(Debug, Deserialize)]
pub struct KeyEntry {
    #[serde(rename = "shareableViewingKey")]
    pub shareable_viewing_key: String,
    #[serde(rename = "railgunAddress")]
    pub railgun_address: String,
}

pub fn load_fixture() -> Fixture {
    let path = repo_root().join("out/logs.json");
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&data).expect("parse fixture")
}

pub fn load_keys() -> HashMap<String, KeyEntry> {
    let path = repo_root().join("out/keys.json");
    let data = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    serde_json::from_str(&data).expect("parse keys")
}

/// The local KeyService wire shape: `base64(viewingKeyHex)`.
pub fn key_ciphertext(shareable_viewing_key: &str) -> String {
    base64::engine::general_purpose::STANDARD.encode(shareable_viewing_key.as_bytes())
}
