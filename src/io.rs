use std::io;

use tokio::net::TcpStream;

pub(crate) async fn write_with<R>(
    stream: &TcpStream,
    mut op: impl FnMut() -> io::Result<R>,
) -> io::Result<R> {
    loop {
        match op() {
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {},
            res => return res,
        }
        stream.writable().await?;
    }
}

macro_rules! ready {
    ($e:expr $(,)?) => {
        match $e {
            ::std::task::Poll::Ready(t) => t,
            ::std::task::Poll::Pending => return ::std::task::Poll::Pending,
        }
    };
}
