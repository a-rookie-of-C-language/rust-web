use spring_macro::Component;

#[Component(scope = "request")]
#[derive(Default)]
struct InvalidScope;

fn main() {}
