use std::net::TcpListener;
use std::sync::Arc;

use spring_context::context::application_context::ApplicationContext;

use crate::request::HttpRequest;
use crate::router::Router;

/// HTTP 服务器
///
/// blocking/std 与 tokio 两种启动入口。
///
/// - `run(...)`：基于 `std::net::TcpListener`，单线程阻塞循环。
/// - `run_tokio(...)`：基于 Tokio socket，多线程 runtime + 每连接一个任务；handler/IoC 仍按当前同步模型执行。
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

        let router = Router::from_registry();

        println!("[spring-web] ┌─────────────────────────────────────────┐");
        println!(
            "[spring-web] │  Server started on http://localhost:{}  │",
            port
        );
        println!("[spring-web] └─────────────────────────────────────────┘");

        for stream in listener.incoming() {
            match stream {
                Ok(mut tcp_stream) => match HttpRequest::parse(&mut tcp_stream) {
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

                        let resp = router.dispatch(&mut req, &context);

                        println!("[spring-web] → {} ({}B body)", resp.status, resp.body.len());

                        if let Err(e) = resp.write_to(&mut tcp_stream) {
                            eprintln!("[spring-web] write error: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("[spring-web] parse error: {}", e);
                    }
                },
                Err(e) => {
                    eprintln!("[spring-web] accept error: {}", e);
                }
            }
        }
    }

    pub fn run_tokio<C: ApplicationContext + 'static>(port: u16, context: C) {
        let rt = tokio::runtime::Builder::new_multi_thread()
            .enable_io()
            .build();

        let rt = match rt {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("[spring-web] failed to build tokio runtime: {}", e);
                return;
            }
        };

        rt.block_on(async move {
            if let Err(e) = Self::run_tokio_async(port, context).await {
                eprintln!("[spring-web] tokio server error: {}", e);
            }
        });
    }

    async fn run_tokio_async<C: ApplicationContext + 'static>(
        port: u16,
        context: C,
    ) -> Result<(), String> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = tokio::net::TcpListener::bind(&addr)
            .await
            .map_err(|e| format!("failed to bind {} — {}", addr, e))?;

        let router = Arc::new(Router::from_registry());
        let context = Arc::new(context);

        println!("[spring-web] ┌─────────────────────────────────────────┐");
        println!(
            "[spring-web] │  Tokio server started on http://localhost:{}  │",
            port
        );
        println!("[spring-web] └─────────────────────────────────────────┘");

        loop {
            let (mut stream, peer) = listener
                .accept()
                .await
                .map_err(|e| format!("accept error: {}", e))?;

            let router = Arc::clone(&router);
            let context = Arc::clone(&context);
            let peer_str = peer.to_string();

            tokio::spawn(async move {
                match HttpRequest::parse_async(&mut stream).await {
                    Ok(mut req) => {
                        println!("[spring-web] {} {} from {}", req.method, req.path, peer_str);

                        let resp = router.dispatch(&mut req, context.as_ref());

                        println!("[spring-web] → {} ({}B body)", resp.status, resp.body.len());

                        if let Err(e) = resp.write_to_async(&mut stream).await {
                            eprintln!("[spring-web] write error: {}", e);
                        }
                    }
                    Err(e) => {
                        eprintln!("[spring-web] parse error: {}", e);
                    }
                }
            });
        }
    }
}
