use ontopolis_accelerate::Accelerator;

/// CUDA accelerator backend.
pub struct CudaAccelerator;

impl Accelerator for CudaAccelerator {
    fn name(&self) -> &str {
        "cuda"
    }

    fn available(&self) -> bool {
        false
    }
}
