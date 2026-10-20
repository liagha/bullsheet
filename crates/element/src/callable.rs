pub trait Callable: 'static {
    fn name(&self) -> crate::Text;

    fn lazy(&self) -> bool {
        false
    }

    fn call(&self, arguments: &[crate::Value]) -> Result<crate::Value, crate::Failure>;
}
