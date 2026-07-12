/// Accelerator capability trait.
pub trait Accelerator {
    fn name(&self) -> &str;
    fn available(&self) -> bool;
}
