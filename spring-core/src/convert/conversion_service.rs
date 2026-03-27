pub trait ConversionService {
    fn can_convert(&self, source_type: &str, target_type: &str) -> bool;
    fn convert_to_string(&self, input: &dyn std::any::Any) -> Option<String>;
}
