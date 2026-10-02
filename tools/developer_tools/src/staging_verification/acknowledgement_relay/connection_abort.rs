//! Connections the relay can end without writing another byte.
//!
//! - **Role:** [`AbortableListener`] accepts the relay's TCP connections for axum and wraps each
//!   in an [`AbortableStream`], whose [`ConnectionAbort`] handle reaches the exchange as its
//!   connection info. Once the handle is pulled, every read, write and flush on that connection
//!   fails, so hyper ends the connection without sending the answer it was handed.
//! - **Position:** under [`super::relay`], between the loopback `TcpListener` and axum's serve
//!   loop.
//! - **Signals & state:** one flag per connection, set once and never cleared.
//! - **Invariants:** after [`ConnectionAbort::abort`], no byte reaches the socket; the socket
//!   closes when hyper drops the connection.

use std::io;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::task::{Context, Poll};

use axum::extract::connect_info::Connected;
use axum::serve::{IncomingStream, Listener};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream};

/// The handle that ends one connection; every exchange on the connection holds a copy.
#[derive(Debug, Clone, Default)]
pub(super) struct ConnectionAbort(Arc<AtomicBool>);

impl ConnectionAbort {
    /// Fail every later read, write and flush on the connection.
    pub(super) fn abort(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    fn is_aborted(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

impl Connected<IncomingStream<'_, AbortableListener>> for ConnectionAbort {
    fn connect_info(stream: IncomingStream<'_, AbortableListener>) -> Self {
        stream.io().abort.clone()
    }
}

/// A TCP listener whose connections carry a [`ConnectionAbort`] handle.
#[derive(Debug)]
pub(super) struct AbortableListener(TcpListener);

impl AbortableListener {
    /// Accept on `listener`.
    pub(super) fn new(listener: TcpListener) -> Self {
        Self(listener)
    }
}

impl Listener for AbortableListener {
    type Io = AbortableStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        // axum's listener retries a failed accept after a pause.
        let (stream, peer) = Listener::accept(&mut self.0).await;
        let stream = AbortableStream {
            stream,
            abort: ConnectionAbort::default(),
        };
        (stream, peer)
    }

    fn local_addr(&self) -> io::Result<Self::Addr> {
        self.0.local_addr()
    }
}

/// One accepted connection and its abort handle.
#[derive(Debug)]
pub(super) struct AbortableStream {
    stream: TcpStream,
    abort: ConnectionAbort,
}

impl AbortableStream {
    fn aborted() -> io::Error {
        io::Error::new(
            io::ErrorKind::ConnectionAborted,
            "the relay withheld the answer on this connection",
        )
    }
}

impl AsyncRead for AbortableStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        if self.abort.is_aborted() {
            return Poll::Ready(Err(Self::aborted()));
        }
        Pin::new(&mut self.stream).poll_read(context, buffer)
    }
}

impl AsyncWrite for AbortableStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        context: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<io::Result<usize>> {
        if self.abort.is_aborted() {
            return Poll::Ready(Err(Self::aborted()));
        }
        Pin::new(&mut self.stream).poll_write(context, bytes)
    }

    fn poll_flush(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        if self.abort.is_aborted() {
            return Poll::Ready(Err(Self::aborted()));
        }
        Pin::new(&mut self.stream).poll_flush(context)
    }

    /// Shutting down sends only the FIN, never buffered bytes, so it proceeds after an abort.
    fn poll_shutdown(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream).poll_shutdown(context)
    }
}
