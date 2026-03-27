use spring_boot::web::{HttpRequest, HttpResponse};
/// REST API 演示 —— 从零 TCP 实现的 HTTP 服务器
///
/// 运行方式：
///   cd example && cargo run --bin web-demo
///
/// 接口：
///   GET  /health                → {"status":"ok"}
///   GET  /products              → 所有商品 JSON 数组
///   GET  /products/{id}         → 单个商品
///   POST /products              → 创建商品（JSON body）
///   PUT  /products/{id}         → 更新商品
///   DELETE /products/{id}       → 删除商品
///
/// curl 测试：
///   curl -s http://localhost:8080/health
///   curl -s http://localhost:8080/products
///   curl -s -X POST http://localhost:8080/products \
///        -H "Content-Type: application/json" \
///        -d '{"name":"Rust Book","price":39.9,"stock":100}'
///   curl -s http://localhost:8080/products/1
///   curl -s -X PUT  http://localhost:8080/products/1 \
///        -d '{"name":"Rust Book 2nd Ed","price":45.0,"stock":80}'
///   curl -s -X DELETE http://localhost:8080/products/1
use spring_boot::{
    Application, ApplicationContext, DeleteMapping, GetMapping, HttpServer, PostMapping,
    PutMapping, Repository,
};

// ── 实体 ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Product {
    name: String,
    price: f64,
    stock: u32,
}

impl Product {
    fn new(name: &str, price: f64, stock: u32) -> Self {
        Self {
            name: name.to_string(),
            price,
            stock,
        }
    }

    fn to_json(&self, id: u64) -> String {
        format!(
            r#"{{"id":{},"name":"{}","price":{},"stock":{}}}"#,
            id, self.name, self.price, self.stock
        )
    }
}

// ── Repository ────────────────────────────────────────────────────────────────

/// #[Repository(Product)] 由宏自动生成内存 CRUD + IoC 注册
#[Repository(Product)]
struct ProductRepository;

// ── 普通路由（无 IoC bean）────────────────────────────────────────────────────

#[GetMapping("/health")]
fn health(_req: &HttpRequest) -> HttpResponse {
    HttpResponse::ok().json(r#"{"status":"ok","server":"spring-web/0.1"}"#)
}

// ── 带 Repository 注入的路由 ──────────────────────────────────────────────────
//
// 宏从参数类型 `ProductRepository` 推导 bean_name = "productRepository"

/// GET /products — 所有商品
#[GetMapping("/products")]
fn list_products(repo: &ProductRepository, _req: &HttpRequest) -> HttpResponse {
    let all = repo.find_all_cloned();
    let items: Vec<String> = all.iter().map(|(id, p)| p.to_json(*id)).collect();
    HttpResponse::ok().json(format!("[{}]", items.join(",")))
}

/// GET /products/{id}
#[GetMapping("/products/{id}")]
fn get_product(repo: &ProductRepository, req: &HttpRequest) -> HttpResponse {
    let id: u64 = req.path_param("id").unwrap_or("0").parse().unwrap_or(0);
    repo.find_by_id(id, |p| match p {
        Some(p) => HttpResponse::ok().json(p.to_json(id)),
        None => {
            HttpResponse::not_found().json(format!(r#"{{"error":"product {} not found"}}"#, id))
        }
    })
}

/// POST /products  body: {"name":"…","price":9.9,"stock":50}
#[PostMapping("/products")]
fn create_product(repo: &ProductRepository, req: &HttpRequest) -> HttpResponse {
    let body = req.body_str();
    match parse_product_json(body) {
        Some(p) => {
            let id = repo.save(p);
            repo.find_by_id(id, |prod| {
                HttpResponse::created().json(prod.unwrap().to_json(id))
            })
        }
        None => HttpResponse::bad_request().json(
            r#"{"error":"invalid JSON, expected {\"name\":\"…\",\"price\":9.9,\"stock\":50}"}"#,
        ),
    }
}

/// PUT /products/{id}  body: {"name":"…","price":9.9,"stock":50}
#[PutMapping("/products/{id}")]
fn update_product(repo: &ProductRepository, req: &HttpRequest) -> HttpResponse {
    let id: u64 = req.path_param("id").unwrap_or("0").parse().unwrap_or(0);
    match parse_product_json(req.body_str()) {
        Some(p) => {
            if repo.update(id, p) {
                repo.find_by_id(id, |prod| {
                    HttpResponse::ok().json(prod.unwrap().to_json(id))
                })
            } else {
                HttpResponse::not_found().json(format!(r#"{{"error":"product {} not found"}}"#, id))
            }
        }
        None => HttpResponse::bad_request().json(r#"{"error":"invalid JSON body"}"#),
    }
}

/// DELETE /products/{id}
#[DeleteMapping("/products/{id}")]
fn delete_product(repo: &ProductRepository, req: &HttpRequest) -> HttpResponse {
    let id: u64 = req.path_param("id").unwrap_or("0").parse().unwrap_or(0);
    if repo.delete_by_id(id) {
        HttpResponse::no_content()
    } else {
        HttpResponse::not_found().json(format!(r#"{{"error":"product {} not found"}}"#, id))
    }
}

// ── 简单 JSON 工具 ─────────────────────────────────────────────────────────────

/// 从 JSON 字符串中提取指定 key 的值（字符串或数字）。
/// 不依赖任何外部 crate，仅用于演示目的。
fn json_field<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pattern = format!("\"{}\"", key);
    let key_pos = json.find(pattern.as_str())?;
    let after_key = json[key_pos + pattern.len()..].trim_start();
    let after_colon = after_key.strip_prefix(':')?.trim_start();
    if let Some(inner) = after_colon.strip_prefix('"') {
        // 字符串值
        let end = inner.find('"')?;
        Some(&inner[..end])
    } else {
        // 数字 / bool / null
        let end = after_colon
            .find([',', '}', ' ', '\n'])
            .unwrap_or(after_colon.len());
        Some(after_colon[..end].trim())
    }
}

fn parse_product_json(json: &str) -> Option<Product> {
    let name = json_field(json, "name")?;
    let price: f64 = json_field(json, "price")?.parse().ok()?;
    let stock: u32 = json_field(json, "stock")?.parse().ok()?;
    Some(Product::new(name, price, stock))
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    println!("=== spring-web demo ===");
    println!("Starting IoC container...");

    // 1. 启动 IoC 容器（注册所有 #[Component] / #[Repository] bean）
    let context = Application::run();

    println!("\nSeeding initial products...");
    // 2. 手动向 repository 写入初始数据（通过 context.get_bean）
    if let Some(bean) = context.get_bean("productRepository") {
        if let Some(repo) = bean.downcast_ref::<ProductRepository>() {
            repo.save(Product::new("Rust Programming Language", 39.9, 100));
            repo.save(Product::new("Cargo Mug", 9.9, 50));
            repo.save(Product::new("Ferris Plush", 19.9, 200));
            println!("  Seeded {} products.", repo.count());
        }
    }

    // 3. 启动 HTTP 服务（阻塞）
    HttpServer::run(8080, context);
}
