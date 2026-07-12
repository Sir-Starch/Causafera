pub struct TerrainCell {
    pub elevation: f32,
}

impl TerrainCell {
    pub fn new(elevation: f32) -> Self {
        Self { elevation }
    }
}
