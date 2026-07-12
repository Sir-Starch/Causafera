/// Local mana field state.
pub struct ManaField {
    pub field_values: Vec<f32>,
}

impl ManaField {
    pub fn new(size: usize) -> Self {
        Self {
            field_values: vec![0.0; size],
        }
    }
}
