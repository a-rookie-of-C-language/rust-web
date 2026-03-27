use spring_macro::GetMapping;

#[GetMapping("/x")]
fn invalid_handler(a: i32, b: i32, c: i32) {}

fn main() {}
