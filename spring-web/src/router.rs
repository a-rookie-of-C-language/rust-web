use std::collections::HashMap;

use spring_context::context::application_context::ApplicationContext;

use crate::method::HttpMethod;
use crate::request::HttpRequest;
use crate::response::HttpResponse;

// ─────────────────────────────────────────────────────────────────────────────
// Handler 类型
// ─────────────────────────────────────────────────────────────────────────────

/// 无 bean 的普通路由处理函数: fn(req) -> HttpResponse
pub type PlainHandlerFn = fn(&HttpRequest) -> HttpResponse;

/// 带 bean 注入的路由处理函数: fn(req, bean_any) -> HttpResponse
pub type BeanHandlerFn = fn(&HttpRequest, &(dyn std::any::Any + Send + Sync)) -> HttpResponse;

/// 路由处理器的两种形式
pub enum Handler {
    /// 普通函数，不依赖 IoC bean
    Plain(PlainHandlerFn),
    /// 需要从 IoC 容器取 bean 后调用
    WithBean {
        bean_name: &'static str,
        f: BeanHandlerFn,
    },
}

// ─────────────────────────────────────────────────────────────────────────────
// RouteRegistration – inventory 收集的路由记录
// ─────────────────────────────────────────────────────────────────────────────

/// 一条路由记录，通过 `inventory::submit!` 在编译期注册，
/// 由 `#[GetMapping]`、`#[PostMapping]` 等宏生成。
pub struct RouteRegistration {
    pub method: HttpMethod,
    /// URL 模式，支持路径参数 `{name}`，例如 `/users/{id}`
    pub path: &'static str,
    pub handler: Handler,
}

inventory::collect!(RouteRegistration);

// ─────────────────────────────────────────────────────────────────────────────
// Router – 运行时路由表
// ─────────────────────────────────────────────────────────────────────────────

pub struct Router {
    routes: Vec<RouteRegistration>,
}

impl Router {
    /// 从 inventory 中收集所有路由记录，构建路由表。
    pub fn from_registry() -> Self {
        let mut routes: Vec<RouteRegistration> = Vec::new();
        for reg in inventory::iter::<RouteRegistration> {
            println!("[spring-web] registered route: {} {}", reg.method, reg.path);
            // inventory::iter 返回引用，但 RouteRegistration 本身是 'static 数据；
            // 为了拿所有权，我们重新包装（Handler 中的函数指针是 Copy）
            routes.push(RouteRegistration {
                method: reg.method.clone(),
                path: reg.path,
                handler: match &reg.handler {
                    Handler::Plain(f) => Handler::Plain(*f),
                    Handler::WithBean { bean_name, f } => Handler::WithBean { bean_name, f: *f },
                },
            });
        }
        Self { routes }
    }

    /// 根据请求匹配路由，调用 handler，返回响应。
    /// 若找不到路由，返回 404；若 bean 不存在，返回 500。
    pub fn dispatch(
        &self,
        req: &mut HttpRequest,
        context: &dyn ApplicationContext,
    ) -> HttpResponse {
        for route in &self.routes {
            if route.method != req.method {
                continue;
            }
            if let Some(params) = match_path(route.path, &req.path) {
                req.path_params = params; // 填充路径参数
                return match &route.handler {
                    Handler::Plain(f) => f(req),
                    Handler::WithBean { bean_name, f } => match context.get_bean(bean_name) {
                        Some(bean) => f(req, bean.as_ref()),
                        None => HttpResponse::internal_error().text(format!(
                            "[spring-web] bean '{}' not found in IoC container",
                            bean_name
                        )),
                    },
                };
            }
        }

        // 检查路径存在但方法不对 → 405
        let path_matched = self
            .routes
            .iter()
            .any(|r| match_path(r.path, &req.path).is_some());
        if path_matched {
            HttpResponse::method_not_allowed().text(format!(
                "405 Method Not Allowed: {} {}",
                req.method, req.path
            ))
        } else {
            HttpResponse::not_found().text(format!("404 Not Found: {} {}", req.method, req.path))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 路径匹配
// ─────────────────────────────────────────────────────────────────────────────

/// 将 URL 模式（如 `/users/{id}/profile`）与实际路径匹配。
/// 成功返回路径参数字典；不匹配返回 None。
///
/// 规则：
/// - 字面量段必须完全相同
/// - `{name}` 段匹配任意非空字符串，并提取为对应参数
fn match_path(pattern: &str, actual: &str) -> Option<HashMap<String, String>> {
    let pp: Vec<&str> = pattern.split('/').collect();
    let ap: Vec<&str> = actual.split('/').collect();

    if pp.len() != ap.len() {
        return None;
    }

    let mut params = HashMap::new();
    for (p, a) in pp.iter().zip(ap.iter()) {
        if p.starts_with('{') && p.ends_with('}') {
            let key = &p[1..p.len() - 1];
            if a.is_empty() {
                return None; // 路径参数不允许为空
            }
            params.insert(key.to_string(), a.to_string());
        } else if p != a {
            return None; // 字面量不匹配
        }
    }
    Some(params)
}

// ─────────────────────────────────────────────────────────────────────────────
// 测试
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use std::any::Any;
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::match_path;
    use super::{Handler, RouteRegistration, Router};
    use crate::method::HttpMethod;
    use crate::request::HttpRequest;
    use crate::response::HttpResponse;
    use spring_context::context::application_context::{ApplicationContext, SharedBean};

    #[derive(Default)]
    struct TestContext {
        beans: HashMap<String, SharedBean>,
    }

    impl ApplicationContext for TestContext {
        fn get_bean(&self, name: &str) -> Option<SharedBean> {
            self.beans.get(name).cloned()
        }

        fn is_singleton(&self, _name: &str) -> bool {
            true
        }

        fn contains_bean(&self, name: &str) -> bool {
            self.beans.contains_key(name)
        }

        fn do_create_bean(&self, _name: &str) -> Option<SharedBean> {
            None
        }
    }

    fn make_request(method: HttpMethod, path: &str) -> HttpRequest {
        HttpRequest {
            method,
            path: path.to_string(),
            query: HashMap::new(),
            headers: HashMap::new(),
            body: Vec::new(),
            path_params: HashMap::new(),
        }
    }

    fn plain_hello(_req: &HttpRequest) -> HttpResponse {
        HttpResponse::ok().text("hello")
    }

    fn with_number(
        _req: &HttpRequest,
        bean: &(dyn Any + Send + Sync),
    ) -> HttpResponse {
        let Some(v) = bean.downcast_ref::<u32>() else {
            return HttpResponse::internal_error().text("type mismatch");
        };
        HttpResponse::ok().text(format!("num={}", v))
    }

    #[test]
    fn test_exact_match() {
        let m = match_path("/users", "/users");
        assert!(m.is_some());
        assert!(m.unwrap().is_empty());
    }

    #[test]
    fn test_no_match() {
        assert!(match_path("/users", "/products").is_none());
    }

    #[test]
    fn test_path_param() {
        let m = match_path("/users/{id}", "/users/42").unwrap();
        assert_eq!(m.get("id"), Some(&"42".to_string()));
    }

    #[test]
    fn test_multi_param() {
        let m = match_path("/users/{uid}/orders/{oid}", "/users/1/orders/99").unwrap();
        assert_eq!(m.get("uid"), Some(&"1".to_string()));
        assert_eq!(m.get("oid"), Some(&"99".to_string()));
    }

    #[test]
    fn test_length_mismatch() {
        assert!(match_path("/users/{id}", "/users/1/extra").is_none());
    }

    #[test]
    fn test_dispatch_plain_handler_ok() {
        let router = Router {
            routes: vec![RouteRegistration {
                method: HttpMethod::GET,
                path: "/hello",
                handler: Handler::Plain(plain_hello),
            }],
        };
        let mut req = make_request(HttpMethod::GET, "/hello");
        let ctx = TestContext::default();
        let resp = router.dispatch(&mut req, &ctx);
        assert_eq!(resp.status.0, 200);
        assert_eq!(resp.body, b"hello");
    }

    #[test]
    fn test_dispatch_with_bean_ok() {
        let router = Router {
            routes: vec![RouteRegistration {
                method: HttpMethod::GET,
                path: "/n",
                handler: Handler::WithBean {
                    bean_name: "n",
                    f: with_number,
                },
            }],
        };
        let mut req = make_request(HttpMethod::GET, "/n");
        let mut ctx = TestContext::default();
        ctx.beans.insert("n".to_string(), Arc::new(7u32));
        let resp = router.dispatch(&mut req, &ctx);
        assert_eq!(resp.status.0, 200);
        assert_eq!(resp.body, b"num=7");
    }

    #[test]
    fn test_dispatch_with_missing_bean_returns_500() {
        let router = Router {
            routes: vec![RouteRegistration {
                method: HttpMethod::GET,
                path: "/needs",
                handler: Handler::WithBean {
                    bean_name: "missing",
                    f: with_number,
                },
            }],
        };
        let mut req = make_request(HttpMethod::GET, "/needs");
        let ctx = TestContext::default();
        let resp = router.dispatch(&mut req, &ctx);
        assert_eq!(resp.status.0, 500);
    }

    #[test]
    fn test_dispatch_method_not_allowed() {
        let router = Router {
            routes: vec![RouteRegistration {
                method: HttpMethod::GET,
                path: "/hello",
                handler: Handler::Plain(plain_hello),
            }],
        };
        let mut req = make_request(HttpMethod::POST, "/hello");
        let ctx = TestContext::default();
        let resp = router.dispatch(&mut req, &ctx);
        assert_eq!(resp.status.0, 405);
    }

    #[test]
    fn test_dispatch_not_found() {
        let router = Router {
            routes: vec![RouteRegistration {
                method: HttpMethod::GET,
                path: "/hello",
                handler: Handler::Plain(plain_hello),
            }],
        };
        let mut req = make_request(HttpMethod::GET, "/missing");
        let ctx = TestContext::default();
        let resp = router.dispatch(&mut req, &ctx);
        assert_eq!(resp.status.0, 404);
    }

    #[test]
    fn test_dispatch_populates_path_params() {
        let router = Router {
            routes: vec![RouteRegistration {
                method: HttpMethod::GET,
                path: "/users/{id}",
                handler: Handler::Plain(plain_hello),
            }],
        };
        let mut req = make_request(HttpMethod::GET, "/users/42");
        let ctx = TestContext::default();
        let _ = router.dispatch(&mut req, &ctx);
        assert_eq!(req.path_param("id"), Some("42"));
    }
}
