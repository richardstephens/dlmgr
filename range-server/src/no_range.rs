use axum::Router;
use axum::body::Body;
use axum::extract::State;
use axum::response::Response;
use axum::routing::get;
use rand::TryRngCore;
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};
use std::net::{Ipv4Addr, SocketAddrV4};
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct AppState {
    data: Arc<Vec<u8>>,
}

pub struct NoRangeServer {
    shutdown_tx: Option<oneshot::Sender<()>>,
    pub url: String,
    pub sha256: Vec<u8>,
    pub join_handle: Option<JoinHandle<()>>,
}

fn make_data(len: usize) -> Vec<u8> {
    let mut buf = vec![0_u8; len];
    OsRng.try_fill_bytes(&mut buf).unwrap();
    buf
}

pub async fn start_no_range_server(len: usize) -> NoRangeServer {
    let data = make_data(len);
    let mut hasher = Sha256::new();
    hasher.update(&data);
    let sha256 = hasher.finalize().to_vec();

    let port = portpicker::pick_unused_port().unwrap();
    let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    let listener = TcpListener::bind(addr).await.unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let app_state = AppState {
        data: Arc::new(data),
    };
    let join_handle = tokio::task::spawn(spawn_axum(listener, app_state, shutdown_rx));

    NoRangeServer {
        shutdown_tx: Some(shutdown_tx),
        url: format!("http://127.0.0.1:{}/test.dat", port),
        sha256,
        join_handle: Some(join_handle),
    }
}

impl Drop for NoRangeServer {
    fn drop(&mut self) {
        self.shutdown_tx.take().unwrap().send(()).unwrap();
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

async fn serve_file(State(state): State<AppState>) -> Response {
    Response::builder()
        .header("Content-Length", state.data.len())
        .body(Body::from(state.data.as_ref().clone()))
        .unwrap()
}
