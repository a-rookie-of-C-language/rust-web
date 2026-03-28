use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpStream;

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, BufReader as TokioBufReader};

use crate::method::HttpMethod;

/// 一个完整的 HTTP 请求
#[derive(Debug)]
pub struct HttpRequest {
    /// 方法（GET / POST …）
    pub method: HttpMethod,
    /// URL 路径（不含 query string）
    pub path: String,
    /// Query 参数（?key=val&…）
    pub query: HashMap<String, String>,
    /// 请求头（全部小写键）
    pub headers: HashMap<String, String>,
    /// 请求体原始字节
    pub body: Vec<u8>,
    /// 路径参数，由 Router 在匹配后填充（如 /users/{id}）
    pub(crate) path_params: HashMap<String, String>,
}

impl HttpRequest {
    /// 从 TcpStream 读取并解析一个 HTTP/1.x 请求。
    /// 使用 BufReader 逐行读取头部，然后按 Content-Length 读取 body。
    pub fn parse(stream: &mut TcpStream) -> Result<Self, String> {
        let mut reader = BufReader::new(stream as &mut dyn Read);
        Self::parse_from_blocking_reader(&mut reader)
    }

    /// 从异步 reader 读取并解析一个 HTTP/1.x 请求。
    pub async fn parse_async<R>(reader: &mut R) -> Result<Self, String>
    where
        R: AsyncRead + Unpin,
    {
        let mut reader = TokioBufReader::new(reader);
        Self::parse_from_async_reader(&mut reader).await
    }

    fn parse_from_blocking_reader<R>(reader: &mut BufReader<R>) -> Result<Self, String>
    where
        R: Read + ?Sized,
    {
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .map_err(|e| format!("read request line: {}", e))?;
        let request_line = request_line.trim_end_matches(['\r', '\n']);

        if request_line.is_empty() {
            return Err("empty request line".to_string());
        }

        let mut parts = request_line.splitn(3, ' ');
        let method_str = parts.next().unwrap_or("");
        let full_path = parts.next().unwrap_or("/");

        let method = method_str
            .parse::<HttpMethod>()
            .map_err(|_| format!("unsupported method: {}", method_str))?;

        let (path, query) = Self::split_path_query(full_path);
        let headers = Self::read_headers_blocking(reader)?;
        let body = Self::read_body_blocking(reader, &headers)?;

        Ok(HttpRequest {
            method,
            path,
            query,
            headers,
            body,
            path_params: HashMap::new(),
        })
    }

    async fn parse_from_async_reader<R>(reader: &mut TokioBufReader<R>) -> Result<Self, String>
    where
        R: AsyncRead + Unpin,
    {
        let mut request_line = String::new();
        reader
            .read_line(&mut request_line)
            .await
            .map_err(|e| format!("read request line: {}", e))?;
        let request_line = request_line.trim_end_matches(['\r', '\n']);

        if request_line.is_empty() {
            return Err("empty request line".to_string());
        }

        let mut parts = request_line.splitn(3, ' ');
        let method_str = parts.next().unwrap_or("");
        let full_path = parts.next().unwrap_or("/");

        let method = method_str
            .parse::<HttpMethod>()
            .map_err(|_| format!("unsupported method: {}", method_str))?;

        let (path, query) = Self::split_path_query(full_path);
        let headers = Self::read_headers_async(reader).await?;
        let body = Self::read_body_async(reader, &headers).await?;

        Ok(HttpRequest {
            method,
            path,
            query,
            headers,
            body,
            path_params: HashMap::new(),
        })
    }

    fn read_headers_blocking<R>(reader: &mut BufReader<R>) -> Result<HashMap<String, String>, String>
    where
        R: Read + ?Sized,
    {
        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .map_err(|e| format!("read header: {}", e))?;
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            if let Some(colon) = line.find(':') {
                let key = line[..colon].trim().to_lowercase();
                let value = line[colon + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }
        Ok(headers)
    }

    async fn read_headers_async<R>(reader: &mut TokioBufReader<R>) -> Result<HashMap<String, String>, String>
    where
        R: AsyncRead + Unpin,
    {
        let mut headers = HashMap::new();
        loop {
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|e| format!("read header: {}", e))?;
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                break;
            }
            if let Some(colon) = line.find(':') {
                let key = line[..colon].trim().to_lowercase();
                let value = line[colon + 1..].trim().to_string();
                headers.insert(key, value);
            }
        }
        Ok(headers)
    }

    fn read_body_blocking<R>(reader: &mut BufReader<R>, headers: &HashMap<String, String>) -> Result<Vec<u8>, String>
    where
        R: Read + ?Sized,
    {
        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader
                .read_exact(&mut body)
                .map_err(|e| format!("read body: {}", e))?;
        }
        Ok(body)
    }

    async fn read_body_async<R>(reader: &mut TokioBufReader<R>, headers: &HashMap<String, String>) -> Result<Vec<u8>, String>
    where
        R: AsyncRead + Unpin,
    {
        let content_length: usize = headers
            .get("content-length")
            .and_then(|v| v.trim().parse().ok())
            .unwrap_or(0);

        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            reader
                .read_exact(&mut body)
                .await
                .map_err(|e| format!("read body: {}", e))?;
        }
        Ok(body)
    }

    /// 获取路径参数（由 Router 在路由匹配后填充）。
    pub fn path_param(&self, key: &str) -> Option<&str> {
        self.path_params.get(key).map(|s| s.as_str())
    }

    /// 获取 Query 参数。
    pub fn query_param(&self, key: &str) -> Option<&str> {
        self.query.get(key).map(|s| s.as_str())
    }

    /// 获取请求头（键不区分大小写，内部已统一小写）。
    pub fn header(&self, key: &str) -> Option<&str> {
        self.headers.get(&key.to_lowercase()).map(|s| s.as_str())
    }

    /// 以 UTF-8 字符串形式返回 body。
    pub fn body_str(&self) -> &str {
        std::str::from_utf8(&self.body).unwrap_or("")
    }

    /// Content-Type 是否为 JSON。
    pub fn is_json(&self) -> bool {
        self.header("content-type")
            .map(|ct| ct.contains("application/json"))
            .unwrap_or(false)
    }

    fn split_path_query(full: &str) -> (String, HashMap<String, String>) {
        let (path_str, query_str) = match full.find('?') {
            Some(i) => (&full[..i], &full[i + 1..]),
            None => (full, ""),
        };

        let mut query = HashMap::new();
        for pair in query_str.split('&') {
            if pair.is_empty() {
                continue;
            }
            match pair.find('=') {
                Some(i) => {
                    query.insert(
                        Self::url_decode(&pair[..i]),
                        Self::url_decode(&pair[i + 1..]),
                    );
                }
                None => {
                    query.insert(Self::url_decode(pair), String::new());
                }
            }
        }

        (path_str.to_string(), query)
    }

    /// 简单 URL 解码（替换 %XX 和 +）
    fn url_decode(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' && i + 2 < bytes.len() {
                if let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                    if let Ok(b) = u8::from_str_radix(hex, 16) {
                        out.push(b as char);
                        i += 3;
                        continue;
                    }
                }
            } else if bytes[i] == b'+' {
                out.push(' ');
                i += 1;
                continue;
            }
            out.push(bytes[i] as char);
            i += 1;
        }
        out
    }
}
