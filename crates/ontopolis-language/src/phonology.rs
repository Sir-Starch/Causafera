pub struct PhonemeInventory {
    pub phonemes: Vec<String>,
}

impl PhonemeInventory {
    pub fn new() -> Self {
        Self {
            phonemes: Vec::new(),
        }
    }
}

impl Default for PhonemeInventory {
    fn default() -> Self {
        Self::new()
    }
}
