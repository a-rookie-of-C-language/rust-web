use std::collections::HashMap;
use std::io::Write;
use std::net::TcpStream;

use tokio::io::{AsyncWrite, AsyncWriteExt};

use crate::status::StatusCode;

/// HTTP 响应构建器
#[derive(Debug)]
pub struct HttpResponse {
    pub status: StatusCode,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl HttpResponse {
    // ──────────────────────────────────────────────────────────────────────────
    // 构造器
    // ──────────────────────────────────────────────────────────────────────────

    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: HashMap::new(),
            body: Vec::new(),
        }
    }

    pub fn ok() -> Self {
        Self::new(StatusCode::OK)
    }
    pub fn created() -> Self {
        Self::new(StatusCode::CREATED)
    }
    pub fn no_content() -> Self {
        Self::new(StatusCode::NO_CONTENT)
    }
    pub fn bad_request() -> Self {
        Self::new(StatusCode::BAD_REQUEST)
    }
    pub fn unauthorized() -> Self {
        Self::new(StatusCode::UNAUTHORIZED)
    }
    pub fn forbidden() -> Self {
        Self::new(StatusCode::FORBIDDEN)
    }
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND)
    }
    pub fn method_not_allowed() -> Self {
        Self::new(StatusCode::METHOD_NOT_ALLOWED)
    }
    pub fn internal_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR)
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 链式 builder
    // ──────────────────────────────────────────────────────────────────────────

    /// 设置任意请求头
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// 设置纯文本 body，自动设置 Content-Type 和 Content-Length
    pub fn text(mut self, text: impl Into<String>) -> Self {
        let bytes = text.into().into_bytes();
        self.headers.insert(
            "Content-Type".to_string(),
            "text/plain; charset=utf-8".to_string(),
        );
        self.headers
            .insert("Content-Length".to_string(), bytes.len().to_string());
        self.body = bytes;
        self
    }

    /// 设置 HTML body
    pub fn html(mut self, markup: impl Into<String>) -> Self {
        let bytes = markup.into().into_bytes();
        self.headers.insert(
            "Content-Type".to_string(),
            "text/html; charset=utf-8".to_string(),
        );
        self.headers
            .insert("Content-Length".to_string(), bytes.len().to_string());
        self.body = bytes;
        self
    }

    /// 设置 JSON body（调用者自己序列化字符串，或传 serde_json）
    pub fn json(mut self, payload: impl Into<String>) -> Self {
        let bytes = payload.into().into_bytes();
        self.headers.insert(
            "Content-Type".to_string(),
            "application/json; charset=utf-8".to_string(),
        );
        self.headers
            .insert("Content-Length".to_string(), bytes.len().to_string());
        self.body = bytes;
        self
    }

    /// 设置任意 body 字节
    pub fn body(mut self, bytes: impl Into<Vec<u8>>) -> Self {
        let b = bytes.into();
        self.headers
            .insert("Content-Length".to_string(), b.len().to_string());
        self.body = b;
        self
    }

    // ──────────────────────────────────────────────────────────────────────────
    // 序列化写入
    // ──────────────────────────────────────────────────────────────────────────

    /// 将响应序列化为 HTTP/1.1 报文写入 TcpStream。
    pub fn write_to(&self, stream: &mut TcpStream) -> std::io::Result<()> {
        stream.write_all(&self.encode())?;
        stream.flush()?;
        Ok(())
    }

    /// 将响应序列化为 HTTP/1.1 报文写入异步 writer。
    pub async fn write_to_async<W>(&self, writer: &mut W) -> std::io::Result<()>
    where
        W: AsyncWrite + Unpin,
    {
        writer.write_all(&self.encode()).await?;
        writer.flush().await?;
        Ok(())
    }

    fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(128 + self.body.len());

        let status_line = format!("HTTP/1.1 {} {}\r\n", self.status.0, self.status.reason());
        out.extend_from_slice(status_line.as_bytes());

        out.extend_from_slice(b"Connection: close\r\n");

        for (key, val) in &self.headers {
            let header = format!("{}: {}\r\n", key, val);
            out.extend_from_slice(header.as_bytes());
        }

        out.extend_from_slice(b"\r\n");
        out.extend_from_slice(&self.body);

        out
    }
}
