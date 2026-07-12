pub struct EconomyNode {
    pub material_flow: Vec<MaterialFlow>,
}

pub struct MaterialFlow {
    pub from: u64,
    pub to: u64,
    pub material: u32,
    pub quantity: f64,
}
