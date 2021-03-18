use std::sync::Arc;
use std::task::Context;
use std::time::Duration;
use std::{io, task::Poll};

use ssh2::{BlockDirections, Session};
use tokio::io::Interest;
use tokio::net::TcpStream;
use tokio::task;
use tokio::time;

macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            ::std::task::Poll::Ready(t) => t,
            ::std::task::Poll::Pending => return ::std::task::Poll::Pending,
        }
    };
}

pub(crate) fn poll<R>(
    session: Session,
    stream: Arc<TcpStream>,
    cx: &mut Context,
    mut op: impl FnMut() -> io::Result<R>,
) -> Poll<io::Result<R>> {
    match op() {
        Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
        res => return Poll::Ready(res),
    }

    match session.block_directions() {
        BlockDirections::None => (),
        BlockDirections::Inbound => ready!(stream.poll_read_ready(cx))?,
        BlockDirections::Outbound => ready!(stream.poll_write_ready(cx))?,
        BlockDirections::Both => {
            ready!(stream.poll_read_ready(cx))?;
            ready!(stream.poll_write_ready(cx))?;
        }
    }

    // We probably block other tasks, force stop and sleep 1ms
    let waker = cx.waker().clone();
    task::spawn(async move {
        time::sleep(Duration::from_millis(1)).await;
        waker.wake();
    });

    Poll::Pending
}

pub(crate) async fn with_async<R>(
    session: &Session,
    stream: &TcpStream,
    mut op: impl FnMut() -> io::Result<R>,
) -> io::Result<R> {
    loop {
        match op() {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
            res => return res,
        }

        match session.block_directions() {
            BlockDirections::None => (),
            BlockDirections::Inbound => stream.readable().await?,
            BlockDirections::Outbound => stream.writable().await?,
            BlockDirections::Both => stream
                .ready(Interest::READABLE | Interest::WRITABLE)
                .await
                .map(|_| ())?,
        }

        // We probably block other tasks, stop and sleep 1ms
        time::sleep(Duration::from_millis(1)).await;
    }
}
