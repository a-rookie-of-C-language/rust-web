pub trait Lifecycle {
    fn start(&mut self);
    fn stop(&mut self);
    fn is_running(&self) -> bool;
}
