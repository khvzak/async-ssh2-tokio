use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use ssh2::{File, FileStat, OpenFlags, OpenType, RenameFlags, Sftp};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

use crate::io::write_with;

pub struct AsyncSftp {
    inner: Sftp,
    stream: Arc<TcpStream>,
}

impl AsyncSftp {
    pub(crate) fn from_parts(inner: Sftp, stream: Arc<TcpStream>) -> Self {
        Self { inner, stream }
    }
}

impl AsyncSftp {
    pub async fn open_mode(
        &self,
        filename: &Path,
        flags: OpenFlags,
        mode: i32,
        open_type: OpenType,
    ) -> io::Result<AsyncFile> {
        let inner = &self.inner;

        let ret = write_with(&self.stream, || {
            inner
                .open_mode(filename, flags, mode, open_type)
                .map_err(Into::into)
        })
        .await;

        ret.map(|file| AsyncFile::from_parts(file, self.stream.clone()))
    }

    pub async fn open(&self, filename: &Path) -> io::Result<AsyncFile> {
        let inner = &self.inner;

        let ret = write_with(&self.stream, || inner.open(filename).map_err(Into::into)).await;

        ret.map(|file| AsyncFile::from_parts(file, self.stream.clone()))
    }

    pub async fn create(&self, filename: &Path) -> io::Result<AsyncFile> {
        let inner = &self.inner;

        let ret = write_with(&self.stream, || inner.create(filename).map_err(Into::into)).await;

        ret.map(|file| AsyncFile::from_parts(file, self.stream.clone()))
    }

    pub async fn opendir(&self, dirname: &Path) -> io::Result<AsyncFile> {
        let inner = &self.inner;

        let ret = write_with(&self.stream, || inner.opendir(dirname).map_err(Into::into)).await;

        ret.map(|file| AsyncFile::from_parts(file, self.stream.clone()))
    }

    pub async fn readdir(&self, dirname: &Path) -> io::Result<Vec<(PathBuf, FileStat)>> {
        let inner = &self.inner;

        write_with(&self.stream, || inner.readdir(dirname).map_err(Into::into)).await
    }

    pub async fn mkdir(&self, filename: &Path, mode: i32) -> io::Result<()> {
        let inner = &self.inner;

        write_with(&self.stream, || {
            inner.mkdir(filename, mode).map_err(Into::into)
        })
        .await
    }

    pub async fn rmdir(&self, filename: &Path) -> io::Result<()> {
        let inner = &self.inner;

        write_with(&self.stream, || inner.rmdir(filename).map_err(Into::into)).await
    }

    pub async fn stat(&self, filename: &Path) -> io::Result<FileStat> {
        let inner = &self.inner;

        write_with(&self.stream, || inner.stat(filename).map_err(Into::into)).await
    }

    pub async fn lstat(&self, filename: &Path) -> io::Result<FileStat> {
        let inner = &self.inner;

        write_with(&self.stream, || inner.lstat(filename).map_err(Into::into)).await
    }

    pub async fn setstat(&self, filename: &Path, stat: FileStat) -> io::Result<()> {
        let inner = &self.inner;

        write_with(&self.stream, || {
            inner.setstat(filename, stat.clone()).map_err(Into::into)
        })
        .await
    }

    pub async fn symlink(&self, path: &Path, target: &Path) -> io::Result<()> {
        let inner = &self.inner;

        write_with(&self.stream, || {
            inner.symlink(path, target).map_err(Into::into)
        })
        .await
    }

    pub async fn readlink(&self, path: &Path) -> io::Result<PathBuf> {
        let inner = &self.inner;

        write_with(&self.stream, || inner.readlink(path).map_err(Into::into)).await
    }

    pub async fn realpath(&self, path: &Path) -> io::Result<PathBuf> {
        let inner = &self.inner;

        write_with(&self.stream, || inner.realpath(path).map_err(Into::into)).await
    }

    pub async fn rename(
        &self,
        src: &Path,
        dst: &Path,
        flags: Option<RenameFlags>,
    ) -> io::Result<()> {
        let inner = &self.inner;

        write_with(&self.stream, || {
            inner.rename(src, dst, flags).map_err(Into::into)
        })
        .await
    }

    pub async fn unlink(&self, file: &Path) -> io::Result<()> {
        let inner = &self.inner;

        write_with(&self.stream, || inner.unlink(file).map_err(Into::into)).await
    }
}

pub struct AsyncFile {
    inner: File,
    stream: Arc<TcpStream>,
}

impl AsyncFile {
    pub(crate) fn from_parts(inner: File, stream: Arc<TcpStream>) -> Self {
        Self { inner, stream }
    }
}

impl AsyncRead for AsyncFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        loop {
            match self.inner.read(buf.initialize_unfilled()) {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                res => return Poll::Ready(res.map(|n| buf.advance(n))),
            }
            ready!(self.stream.poll_read_ready(cx))?;
        }
    }
}

impl AsyncWrite for AsyncFile {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        loop {
            match self.inner.write(buf) {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                res => return Poll::Ready(res),
            }
            ready!(self.stream.poll_write_ready(cx))?;
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        loop {
            match self.inner.flush() {
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => {}
                res => return Poll::Ready(res),
            }
            ready!(self.stream.poll_write_ready(cx))?;
        }
    }

    fn poll_shutdown(self: Pin<&mut Self>, cx: &mut Context) -> Poll<io::Result<()>> {
        self.poll_flush(cx)
    }
}
