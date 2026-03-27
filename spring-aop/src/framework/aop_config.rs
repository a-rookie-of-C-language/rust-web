/// Global AOP configuration flags.
#[derive(Debug, Clone, Default)]
pub struct AopConfig {
    /// When `true`, the `AopBeanPostProcessor` prints a debug line each time
    /// an advisor is applied to a bean.
    pub debug: bool,
}
