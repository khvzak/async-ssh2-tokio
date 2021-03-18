use std::io::{self, Read, Write};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use ssh2::{Channel, ExitSignal, ExtendedData, PtyModes, ReadWindow, Session, Stream, WriteWindow};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

use crate::io::{poll, with_async};

pub struct AsyncChannel {
    inner: Channel,
    session: Session,
    stream: Arc<TcpStream>,
}

impl AsyncChannel {
    pub(crate) fn from_parts(inner: Channel, session: Session, stream: Arc<TcpStream>) -> Self {
        Self {
            inner,
            session,
            stream,
        }
    }
}

impl AsyncChannel {
    pub async fn setenv(&mut self, var: &str, val: &str) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.setenv(var, val).map_err(Into::into)
        })
        .await
    }

    pub async fn request_pty(
        &mut self,
        term: &str,
        mode: Option<PtyModes>,
        dim: Option<(u32, u32, u32, u32)>,
    ) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner
                .request_pty(term, mode.clone(), dim)
                .map_err(Into::into)
        })
        .await
    }

    pub async fn request_pty_size(
        &mut self,
        width: u32,
        height: u32,
        width_px: Option<u32>,
        height_px: Option<u32>,
    ) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner
                .request_pty_size(width, height, width_px, height_px)
                .map_err(Into::into)
        })
        .await
    }

    pub async fn request_auth_agent_forwarding(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.request_auth_agent_forwarding().map_err(Into::into)
        })
        .await
    }

    pub async fn exec(&mut self, command: &str) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.exec(command).map_err(Into::into)
        })
        .await
    }

    pub async fn shell(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.shell().map_err(Into::into)
        })
        .await
    }

    pub async fn subsystem(&mut self, system: &str) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.subsystem(system).map_err(Into::into)
        })
        .await
    }

    pub async fn process_startup(
        &mut self,
        request: &str,
        message: Option<&str>,
    ) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.process_startup(request, message).map_err(Into::into)
        })
        .await
    }

    pub fn stderr(&self) -> AsyncStream {
        AsyncStream::from_parts(
            self.inner.stderr(),
            self.session.clone(),
            self.stream.clone(),
        )
    }

    pub fn stream(&self, stream_id: i32) -> AsyncStream {
        AsyncStream::from_parts(
            self.inner.stream(stream_id),
            self.session.clone(),
            self.stream.clone(),
        )
    }

    pub async fn handle_extended_data(&mut self, mode: ExtendedData) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.handle_extended_data(mode).map_err(Into::into)
        })
        .await
    }

    pub fn exit_status(&self) -> io::Result<i32> {
        self.inner.exit_status().map_err(Into::into)
    }

    pub async fn exit_signal(&self) -> io::Result<ExitSignal> {
        self.inner.exit_signal().map_err(Into::into)
    }

    pub fn read_window(&self) -> ReadWindow {
        self.inner.read_window()
    }

    pub fn write_window(&self) -> WriteWindow {
        self.inner.write_window()
    }

    pub async fn adjust_receive_window(&mut self, adjust: u64, force: bool) -> io::Result<u64> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner
                .adjust_receive_window(adjust, force)
                .map_err(Into::into)
        })
        .await
    }

    pub fn eof(&self) -> bool {
        self.inner.eof()
    }

    pub async fn send_eof(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.send_eof().map_err(Into::into)
        })
        .await
    }

    pub async fn wait_eof(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.wait_eof().map_err(Into::into)
        })
        .await
    }

    pub async fn close(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.close().map_err(Into::into)
        })
        .await
    }

    pub async fn wait_close(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        with_async(&self.session, &self.stream, || {
            inner.wait_close().map_err(Into::into)
        })
        .await
    }
}

impl AsyncRead for AsyncChannel {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream(0)).poll_read(cx, buf)
    }
}

impl AsyncWrite for AsyncChannel {
    fn poll_write(self: Pin<&mut Self>, cx: &mut Context, buf: &[u8]) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.stream(0)).poll_write(cx, buf)
    }

    fn poll_flush(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        Pin::new(&mut self.stream(0)).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        poll(self.session.clone(), self.stream.clone(), cx, || {
            self.inner.close().map_err(io::Error::from)
        })
    }
}

pub struct AsyncStream {
    inner: Stream,
    session: Session,
    stream: Arc<TcpStream>,
}

impl AsyncStream {
    pub(crate) fn from_parts(inner: Stream, session: Session, stream: Arc<TcpStream>) -> Self {
        Self {
            inner,
            session,
            stream,
        }
    }
}

impl AsyncRead for AsyncStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf,
    ) -> Poll<io::Result<()>> {
        poll(self.session.clone(), self.stream.clone(), cx, || {
            self.inner
                .read(buf.initialize_unfilled())
                .map(|n| buf.advance(n))
        })
    }
}

impl AsyncWrite for AsyncStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        poll(self.session.clone(), self.stream.clone(), cx, || {
            self.inner.write(buf)
        })
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        poll(self.session.clone(), self.stream.clone(), cx, || {
            self.inner.flush()
        })
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
