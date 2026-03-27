//! rust-spring initializer
//!
//! Usage:
//!   initializer --name <project-name> [--output <dir>]
//!
//! Generates a minimal rust-spring project under <output>/<project-name>/.

use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::path::{Path, PathBuf};

// ── entry point ──────────────────────────────────────────────────────────────

fn main() {
    let cfg = match Config::parse(env::args().skip(1).collect()) {
        Ok(c) => c,
        Err(msg) => {
            eprintln!("Error: {msg}");
            eprintln!();
            eprintln!("{}", usage());
            std::process::exit(1);
        }
    };

    if let Err(e) = generate(&cfg) {
        eprintln!("Error generating project: {e}");
        std::process::exit(1);
    }
}

// ── config ────────────────────────────────────────────────────────────────────

struct Config {
    name: String,
    output: PathBuf,
}

impl Config {
    fn parse(args: Vec<String>) -> Result<Self, String> {
        let mut name: Option<String> = None;
        let mut output: Option<PathBuf> = None;
        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--name" | "-n" => {
                    name = Some(iter.next().ok_or("--name requires a value")?);
                }
                "--output" | "-o" => {
                    output = Some(PathBuf::from(
                        iter.next().ok_or("--output requires a value")?,
                    ));
                }
                "--help" | "-h" => {
                    println!("{}", usage());
                    std::process::exit(0);
                }
                other => {
                    if name.is_none() && !other.starts_with('-') {
                        name = Some(other.to_string());
                    } else {
                        return Err(format!("Unknown argument: {other}"));
                    }
                }
            }
        }

        let name = name.ok_or("Project name is required. Use --name <project-name>")?;
        if name.is_empty() {
            return Err("Project name must not be empty".into());
        }
        if !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        {
            return Err("Project name may only contain letters, digits, '-', and '_'".into());
        }

        let output = output.unwrap_or_else(|| PathBuf::from("."));
        Ok(Config { name, output })
    }
}

fn usage() -> String {
    [
        "rust-spring initializer",
        "",
        "USAGE:",
        "    initializer --name <project-name> [--output <dir>]",
        "    initializer <project-name> [--output <dir>]",
        "",
        "OPTIONS:",
        "    -n, --name <name>     Project name (Cargo package name)",
        "    -o, --output <dir>    Directory to create the project in (default: current dir)",
        "    -h, --help            Show this help message",
        "",
        "EXAMPLE:",
        "    cargo run -p initializer -- --name my-app",
        "    cargo run -p initializer -- my-app --output /tmp",
    ]
    .join("\n")
}

// ── generator ─────────────────────────────────────────────────────────────────

fn generate(cfg: &Config) -> Result<(), String> {
    let project_dir = cfg.output.join(&cfg.name);
    let src_dir = project_dir.join("src");

    if project_dir.exists() {
        return Err(format!(
            "Directory '{}' already exists",
            project_dir.display()
        ));
    }

    fs::create_dir_all(&src_dir)
        .map_err(|e| format!("Cannot create {}: {e}", src_dir.display()))?;

    let cargo_toml = render_cargo_toml(&cfg.name);
    let main_rs = render_main_rs(&cfg.name);
    let app_props = render_application_properties(&cfg.name);

    write_file(&project_dir.join("Cargo.toml"), &cargo_toml)?;
    write_file(&src_dir.join("main.rs"), &main_rs)?;
    write_file(&project_dir.join("application.properties"), &app_props)?;

    println!();
    println!("  Created rust-spring project '{}'", cfg.name);
    println!();
    println!("  Structure:");
    println!("    {}/", cfg.name);
    println!("    +-- Cargo.toml");
    println!("    +-- application.properties");
    println!("    +-- src/");
    println!("        +-- main.rs");
    println!();
    println!("  Next steps:");
    println!("    cd {}", project_dir.display());
    println!("    cargo run");
    println!();

    Ok(())
}

// ── template renderers ───────────────────────────────────────────────────────

fn render_cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[dependencies]
spring-boot = {{ git = "https://github.com/a-rookie-of-C-language/rust-spring" }}
"#
    )
}

fn render_main_rs(name: &str) -> String {
    let pascal = to_pascal_case(name);
    let snake = name.replace('-', "_");

    format!(
        r#"use spring_boot::{{Application, ApplicationContext, Component, Value}};

// A simple singleton bean - registered automatically by #[Component].
#[Component]
#[derive(Debug, Default, Clone)]
struct HelloService {{
    #[Value("${{greeting:Hello}}")]
    greeting: String,
}}

impl HelloService {{
    pub fn say_hello(&self, to: &str) {{
        println!("{{}} from {pascal}! (to: {{}})", self.greeting, to);
    }}
}}

// A bean with Value injection from application.properties.
#[Component]
#[derive(Debug, Default, Clone)]
struct AppConfig {{
    #[Value("${{app.name:{snake}}}")]
    app_name: String,

    #[Value("${{server.port:8080}}")]
    port: i32,
}}

fn main() {{
    let context = Application::run();

    if let Some(bean) = context.get_bean("helloService") {{
        if let Some(svc) = bean.downcast_ref::<HelloService>() {{
            svc.say_hello("world");
        }}
    }}

    if let Some(bean) = context.get_bean("appConfig") {{
        if let Some(cfg) = bean.downcast_ref::<AppConfig>() {{
            println!("[Config] app.name   = {{}}", cfg.app_name);
            println!("[Config] server.port = {{}}", cfg.port);
        }}
    }}
}}
"#
    )
}

fn render_application_properties(name: &str) -> String {
    format!(
        r#"# {name} - application configuration
# Values here are injected via #[Value("${{key:default}}")]

app.name={name}
server.port=8080
greeting=Hello, rust-spring
"#
    )
}

// ── helpers ──────────────────────────────────────────────────────────────────

fn to_pascal_case(s: &str) -> String {
    s.split(['-', '_'])
        .filter(|part| !part.is_empty())
        .fold(String::new(), |mut acc, part| {
            let mut chars = part.chars();
            if let Some(first) = chars.next() {
                let _ = write!(acc, "{}{}", first.to_uppercase(), chars.as_str());
            }
            acc
        })
}

fn write_file(path: &Path, content: &str) -> Result<(), String> {
    fs::write(path, content).map_err(|e| format!("Cannot write {}: {e}", path.display()))
}
