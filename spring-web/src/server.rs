use std::net::TcpListener;

use spring_context::context::application_context::ApplicationContext;

use crate::request::HttpRequest;
use crate::router::Router;

/// HTTP 服务器
///
/// 基于 `std::net::TcpListener` 实现，**纯 std**，不依赖任何异步运行时。
/// 采用单线程循环（每次处理一个请求），适合学习和演示场景。
///
/// 使用示例：
/// ```ignore
/// let context = Application::run();
/// HttpServer::run(8080, context);
/// ```
pub struct HttpServer;

impl HttpServer {
    /// 在 `port` 端口启动 HTTP 服务，阻塞直到程序退出。
    ///
    /// `context` 实现了 `ApplicationContext`，用于解析 `WithBean` 路由中的 IoC bean。
    pub fn run<C: ApplicationContext>(port: u16, context: C) {
        let addr = format!("0.0.0.0:{}", port);
        let listener = match TcpListener::bind(&addr) {
            Ok(listener) => listener,
            Err(e) => {
                eprintln!("[spring-web] failed to bind {}:{} — {}", addr, port, e);
                return;
            }
        };

        // 从 inventory 收集所有路由
        let router = Router::from_registry();

        println!("[spring-web] ┌─────────────────────────────────────────┐");
        println!(
            "[spring-web] │  Server started on http://localhost:{}  │",
            port
        );
        println!("[spring-web] └─────────────────────────────────────────┘");

        for stream in listener.incoming() {
            match stream {
                Ok(mut tcp_stream) => {
                    // 解析请求
                    match HttpRequest::parse(&mut tcp_stream) {
                        Ok(mut req) => {
                            println!(
                                "[spring-web] {} {} from {}",
                                req.method,
                                req.path,
                                tcp_stream
                                    .peer_addr()
                                    .map(|a| a.to_string())
                                    .unwrap_or_else(|_| "?".to_string())
                            );

                            // 分发到路由
                            let resp = router.dispatch(&mut req, &context);

                            println!("[spring-web] → {} ({}B body)", resp.status, resp.body.len());

                            // 写回响应
                            if let Err(e) = resp.write_to(&mut tcp_stream) {
                                eprintln!("[spring-web] write error: {}", e);
                            }
                        }
                        Err(e) => {
                            eprintln!("[spring-web] parse error: {}", e);
                        }
                    }
                }
                Err(e) => {
                    eprintln!("[spring-web] accept error: {}", e);
                }
            }
        }
    }
}
