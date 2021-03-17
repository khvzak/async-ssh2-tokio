use std::io;
use std::sync::Arc;
use std::time::Duration;

use ssh2::Listener;
use tokio::net::TcpStream;
use tokio::time;

use crate::channel::AsyncChannel;

pub struct AsyncListener {
    inner: Listener,
    stream: Arc<TcpStream>,
}

impl AsyncListener {
    pub(crate) fn from_parts(inner: Listener, stream: Arc<TcpStream>) -> Self {
        Self { inner, stream }
    }
}

impl AsyncListener {
    pub async fn accept(&mut self) -> io::Result<AsyncChannel> {
        // The I/O object for Listener::accept is on the remote SSH server. There is no way to poll
        // its state so the best we can do is loop and periodically check whether we have a new
        // connection.
        let channel = loop {
            match self.inner.accept() {
                Ok(channel) => break channel,
                Err(e)
                    if io::Error::from(ssh2::Error::from_errno(e.code())).kind()
                        == io::ErrorKind::WouldBlock => {}
                Err(e) => return Err(io::Error::from(e)),
            }

            time::sleep(Duration::from_millis(10)).await;
        };

        Ok(AsyncChannel::from_parts(channel, self.stream.clone()))
    }
}
