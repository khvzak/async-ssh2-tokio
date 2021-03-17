use std::io;
use std::sync::Arc;

use ssh2::{Agent, PublicKey};
use tokio::net::TcpStream;

use crate::io::write_with;
use crate::session::get_session;

pub struct AsyncAgent {
    inner: Agent,
    stream: Arc<TcpStream>,
}

impl AsyncAgent {
    pub fn new(stream: TcpStream) -> io::Result<Self> {
        let stream = Arc::new(stream);

        let mut session = get_session(None)?;

        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            session.set_tcp_stream(stream.as_raw_fd());
        }

        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawSocket;
            session.set_tcp_stream(stream.as_raw_socket());
        }

        let agent = session.agent()?;

        Ok(Self::from_parts(agent, stream))
    }

    pub(crate) fn from_parts(inner: Agent, stream: Arc<TcpStream>) -> Self {
        Self { inner, stream }
    }
}

impl AsyncAgent {
    pub async fn connect(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        write_with(&self.stream, || inner.connect().map_err(Into::into)).await
    }

    pub async fn disconnect(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        write_with(&self.stream, || inner.disconnect().map_err(Into::into)).await
    }

    pub async fn list_identities(&mut self) -> io::Result<()> {
        let inner = &mut self.inner;

        write_with(&self.stream, || inner.list_identities().map_err(Into::into)).await
    }

    pub fn identities(&self) -> io::Result<Vec<PublicKey>> {
        self.inner.identities().map_err(Into::into)
    }

    pub async fn userauth(&self, username: &str, identity: &PublicKey) -> io::Result<()> {
        let inner = &self.inner;

        write_with(&self.stream, || {
            inner.userauth(username, identity).map_err(Into::into)
        })
        .await
    }
}
