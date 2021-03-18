use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use ssh2::{File, FileStat, OpenFlags, OpenType, RenameFlags, Session, Sftp};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;

use crate::io::{poll, with_async};

pub struct AsyncSftp {
    inner: Sftp,
    session: Session,
    stream: Arc<TcpStream>,
}

impl AsyncSftp {
    pub(crate) fn from_parts(inner: Sftp, session: Session, stream: Arc<TcpStream>) -> Self {
        Self {
            inner,
            session,
            stream,
        }
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

        let ret = with_async(&self.session, &self.stream, || {
            inner
                .open_mode(filename, flags, mode, open_type)
                .map_err(Into::into)
        })
        .await;

        ret.map(|file| AsyncFile::from_parts(file, self.session.clone(), self.stream.clone()))
    }

    pub async fn open(&self, filename: &Path) -> io::Result<AsyncFile> {
        let inner = &self.inner;

        let ret = with_async(&self.session, &self.stream, || {
            inner.open(filename).map_err(Into::into)
        })
        .await;

        ret.map(|file| AsyncFile::from_parts(file, self.session.clone(), self.stream.clone()))
    }

    pub async fn create(&self, filename: &Path) -> io::Result<AsyncFile> {
        let inner = &self.inner;

        let ret = with_async(&self.session, &self.stream, || {
            inner.create(filename).map_err(Into::into)
        })
        .await;

        ret.map(|file| AsyncFile::from_parts(file, self.session.clone(), self.stream.clone()))
    }

    pub async fn opendir(&self, dirname: &Path) -> io::Result<AsyncFile> {
        let inner = &self.inner;

        let ret = with_async(&self.session, &self.stream, || {
            inner.opendir(dirname).map_err(Into::into)
        })
        .await;

        ret.map(|file| AsyncFile::from_parts(file, self.session.clone(), self.stream.clone()))
    }

    pub async fn readdir(&self, dirname: &Path) -> io::Result<Vec<(PathBuf, FileStat)>> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.readdir(dirname).map_err(Into::into)
        })
        .await
    }

    pub async fn mkdir(&self, filename: &Path, mode: i32) -> io::Result<()> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.mkdir(filename, mode).map_err(Into::into)
        })
        .await
    }

    pub async fn rmdir(&self, filename: &Path) -> io::Result<()> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.rmdir(filename).map_err(Into::into)
        })
        .await
    }

    pub async fn stat(&self, filename: &Path) -> io::Result<FileStat> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.stat(filename).map_err(Into::into)
        })
        .await
    }

    pub async fn lstat(&self, filename: &Path) -> io::Result<FileStat> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.lstat(filename).map_err(Into::into)
        })
        .await
    }

    pub async fn setstat(&self, filename: &Path, stat: FileStat) -> io::Result<()> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.setstat(filename, stat.clone()).map_err(Into::into)
        })
        .await
    }

    pub async fn symlink(&self, path: &Path, target: &Path) -> io::Result<()> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.symlink(path, target).map_err(Into::into)
        })
        .await
    }

    pub async fn readlink(&self, path: &Path) -> io::Result<PathBuf> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.readlink(path).map_err(Into::into)
        })
        .await
    }

    pub async fn realpath(&self, path: &Path) -> io::Result<PathBuf> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.realpath(path).map_err(Into::into)
        })
        .await
    }

    pub async fn rename(
        &self,
        src: &Path,
        dst: &Path,
        flags: Option<RenameFlags>,
    ) -> io::Result<()> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.rename(src, dst, flags).map_err(Into::into)
        })
        .await
    }

    pub async fn unlink(&self, file: &Path) -> io::Result<()> {
        let inner = &self.inner;

        with_async(&self.session, &self.stream, || {
            inner.unlink(file).map_err(Into::into)
        })
        .await
    }
}

pub struct AsyncFile {
    inner: File,
    session: Session,
    stream: Arc<TcpStream>,
}

impl AsyncFile {
    pub(crate) fn from_parts(inner: File, session: Session, stream: Arc<TcpStream>) -> Self {
        Self {
            inner,
            session,
            stream,
        }
    }
}

impl AsyncRead for AsyncFile {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        poll(self.session.clone(), self.stream.clone(), cx, || {
            self.inner
                .read(buf.initialize_unfilled())
                .map(|n| buf.advance(n))
        })
    }
}

impl AsyncWrite for AsyncFile {
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
