use axum::Router;
use axum::extract::State;
use axum::routing::get;
use axum_range::{AsyncSeekStart, RangeBody, Ranged};
use std::cmp::min;

use std::net::{Ipv4Addr, SocketAddrV4};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};
use tokio::net::TcpListener;
use tokio::sync::oneshot;

use axum_extra::TypedHeader;
use axum_extra::headers::Range;
use tokio::task::JoinHandle;

#[derive(Clone)]
struct AppState {
    zero_len: u64,
}

//TODO: the async trait impls here are hastily-written plane code,
// check that they are correct.
pub struct ZeroServer {
    pub url: String,
    shutdown_tx: Option<oneshot::Sender<()>>,
    pub join_handle: Option<JoinHandle<()>>,
}

pub async fn start_zero_server(zero_len: u64) -> ZeroServer {
    let port = portpicker::pick_unused_port().unwrap();
    let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    let listener = TcpListener::bind(addr).await.unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let app_state = AppState { zero_len };
    let join_handle = tokio::task::spawn(spawn_axum(listener, app_state, shutdown_rx));

    ZeroServer {
        url: format!("http://127.0.0.1:{}/zero", port),
        shutdown_tx: Some(shutdown_tx),
        join_handle: Some(join_handle),
    }
}
impl Drop for ZeroServer {
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
        .route("/zero", get(serve_file))
        .with_state(app_state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        })
        .await
        .unwrap();
}

struct ZeroBody {
    len: u64,
    pos: u64,
}

impl RangeBody for ZeroBody {
    fn byte_size(&self) -> u64 {
        self.len
    }
}
impl AsyncRead for ZeroBody {
    fn poll_read(
        self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if self.pos < self.len {
            let buff_size = min(self.len - self.pos, 32768) as usize;
            buf.put_slice(&vec![0_u8; buff_size]);
        }
        Poll::Ready(Ok(()))
    }
}
impl AsyncSeekStart for ZeroBody {
    fn start_seek(mut self: Pin<&mut Self>, pos: u64) -> std::io::Result<()> {
        self.pos = pos;
        Ok(())
    }

    fn poll_complete(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

async fn serve_file(
    State(state): State<AppState>,
    range: Option<TypedHeader<Range>>,
) -> Ranged<ZeroBody> {
    let range = range.map(|TypedHeader(range)| range);
    Ranged::new(
        range,
        ZeroBody {
            len: state.zero_len,
            pos: 0,
        },
    )
}
