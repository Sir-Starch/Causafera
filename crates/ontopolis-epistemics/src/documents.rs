use ontopolis_types::DocumentId;

pub struct Document {
    pub id: DocumentId,
    pub medium: String,
    pub glyph_sequence: Vec<u32>,
}
