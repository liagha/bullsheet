pub trait Callable: 'static {
    fn name(&self) -> crate::Text;

    fn call(&self, arguments: &[crate::Value]) -> Result<crate::Value, crate::Failure>;
}
