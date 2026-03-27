pub trait Converter<S, T> {
    fn convert(&self, source: S) -> Result<T, String>;
}
