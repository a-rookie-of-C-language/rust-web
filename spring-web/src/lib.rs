//! spring-web — 轻量级 HTTP 服务器
//!
//! 提供：
//! - [`HttpMethod`] / [`StatusCode`] — HTTP 基础类型
//! - [`HttpRequest`] — 从 TCP 流解析 HTTP/1.x 请求（含 path params、query、header、body）
//! - [`HttpResponse`] — 链式构建响应（text / json / html / body）
//! - [`RouteRegistration`] / [`Handler`] — `inventory` 路由注册表
//! - [`Router`] — 路径匹配（支持 `{param}`）+ IoC bean 注入分发
//! - [`HttpServer`] — blocking/std 与 tokio 两种启动入口

pub mod method;
pub mod request;
pub mod response;
pub mod router;
pub mod server;
pub mod status;

pub use method::HttpMethod;
pub use request::HttpRequest;
pub use response::HttpResponse;
pub use router::{BeanHandlerFn, Handler, PlainHandlerFn, RouteRegistration, Router};
pub use server::HttpServer;
pub use status::StatusCode;
