use axum::Router;
use axum::extract::State;
use axum::routing::get;
use axum_range::{KnownSize, Ranged};
use rand::TryRngCore;
use sha2::{Digest, Sha256};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::path::{Path, PathBuf};
use tempfile::TempDir;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use axum_extra::TypedHeader;
use axum_extra::headers::Range;
use rand::rngs::OsRng;
use tokio::fs::File;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct AppState {
    tmpfile_path: PathBuf,
}

pub struct TempServer {
    tempdir: Option<TempDir>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    pub tmpfile_url: String,
    pub tmpfile_sha256: Vec<u8>,
    pub join_handle: Option<JoinHandle<()>>,
}

async fn make_tempfile(path: &Path) -> Vec<u8> {
    let mut file = File::create_new(path).await.unwrap();
    let mut hasher = Sha256::new();

    for _ii in 0..100 {
        let mut rand_buf = vec![0_u8; 128 * 1024];
        OsRng.try_fill_bytes(&mut rand_buf).unwrap();
        hasher.update(&rand_buf);
        file.write_all(&rand_buf).await.unwrap();
    }

    hasher.finalize().to_vec()
}

pub async fn start_temp_server() -> TempServer {
    let tempdir = tempfile::tempdir().unwrap();

    let tmpfile_path = tempdir.path().join("test.dat");
    let tmpfile_sha256 = make_tempfile(&tmpfile_path).await;

    let port = portpicker::pick_unused_port().unwrap();
    let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    let listener = TcpListener::bind(addr).await.unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let app_state = AppState { tmpfile_path };
    let join_handle = tokio::task::spawn(spawn_axum(listener, app_state, shutdown_rx));

    TempServer {
        tempdir: Some(tempdir),
        shutdown_tx: Some(shutdown_tx),
        tmpfile_url: format!("http://127.0.0.1:{}/test.dat", port),
        tmpfile_sha256,
        join_handle: Some(join_handle),
    }
}
impl Drop for TempServer {
    fn drop(&mut self) {
        self.shutdown_tx.take().unwrap().send(()).unwrap();
        self.tempdir.take().unwrap().close().unwrap();
    }
}

async fn spawn_axum(
    listener: TcpListener,
    app_state: AppState,
    shutdown_rx: oneshot::Receiver<()>,
) {
    let app = Router::new()
        .route("/test.dat", get(serve_file))
        .with_state(app_state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        })
        .await
        .unwrap();
}

async fn serve_file(
    State(state): State<AppState>,
    range: Option<TypedHeader<Range>>,
) -> Ranged<KnownSize<File>> {
    let file = File::open(&state.tmpfile_path).await.unwrap();
    let body = KnownSize::file(file).await.unwrap();
    let range = range.map(|TypedHeader(range)| range);
    Ranged::new(range, body)
}
