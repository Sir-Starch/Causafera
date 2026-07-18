use causafera_types::{
    DocumentId, DocumentMediumId, DocumentTransformationId, GlyphId, SimulationTime,
    WritingSystemId,
};

pub const MAX_DOCUMENT_GLYPHS: usize = 4096;
pub const MAX_COPY_EDITS: usize = 128;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GlyphEdit {
    Insert { at: u16, glyph: GlyphId },
    Remove { at: u16 },
    Replace { at: u16, glyph: GlyphId },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DocumentTransformation {
    pub id: DocumentTransformationId,
    pub source: DocumentId,
    pub target: DocumentId,
    pub occurred_at: SimulationTime,
    pub edits: Vec<GlyphEdit>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Document {
    id: DocumentId,
    parent: Option<DocumentId>,
    medium: DocumentMediumId,
    writing_system: WritingSystemId,
    created_at: SimulationTime,
    glyphs: Vec<GlyphId>,
    transformation: Option<DocumentTransformationId>,
}

impl Document {
    pub fn new(
        id: DocumentId,
        medium: DocumentMediumId,
        writing_system: WritingSystemId,
        created_at: SimulationTime,
        glyphs: Vec<GlyphId>,
    ) -> Result<Self, DocumentError> {
        if glyphs.len() > MAX_DOCUMENT_GLYPHS {
            return Err(DocumentError::TooManyGlyphs);
        }
        Ok(Self {
            id,
            parent: None,
            medium,
            writing_system,
            created_at,
            glyphs,
            transformation: None,
        })
    }

    pub const fn id(&self) -> DocumentId {
        self.id
    }

    pub const fn parent(&self) -> Option<DocumentId> {
        self.parent
    }

    pub const fn medium(&self) -> DocumentMediumId {
        self.medium
    }

    pub const fn writing_system(&self) -> WritingSystemId {
        self.writing_system
    }

    pub const fn created_at(&self) -> SimulationTime {
        self.created_at
    }

    pub fn glyphs(&self) -> &[GlyphId] {
        &self.glyphs
    }

    pub const fn transformation(&self) -> Option<DocumentTransformationId> {
        self.transformation
    }

    pub fn copy_with_edits(
        &self,
        target: DocumentId,
        medium: DocumentMediumId,
        occurred_at: SimulationTime,
        transformation_id: DocumentTransformationId,
        edits: Vec<GlyphEdit>,
    ) -> Result<(Self, DocumentTransformation), DocumentError> {
        if target == self.id {
            return Err(DocumentError::InvalidLineage);
        }
        if edits.len() > MAX_COPY_EDITS {
            return Err(DocumentError::TooManyEdits);
        }
        let mut glyphs = self.glyphs.clone();
        for edit in &edits {
            match *edit {
                GlyphEdit::Insert { at, glyph } => {
                    let at = usize::from(at);
                    if at > glyphs.len() || glyphs.len() == MAX_DOCUMENT_GLYPHS {
                        return Err(DocumentError::InvalidEdit);
                    }
                    glyphs.insert(at, glyph);
                }
                GlyphEdit::Remove { at } => {
                    let at = usize::from(at);
                    if at >= glyphs.len() {
                        return Err(DocumentError::InvalidEdit);
                    }
                    glyphs.remove(at);
                }
                GlyphEdit::Replace { at, glyph } => {
                    let Some(slot) = glyphs.get_mut(usize::from(at)) else {
                        return Err(DocumentError::InvalidEdit);
                    };
                    *slot = glyph;
                }
            }
        }
        let transformation = DocumentTransformation {
            id: transformation_id,
            source: self.id,
            target,
            occurred_at,
            edits,
        };
        let document = Self {
            id: target,
            parent: Some(self.id),
            medium,
            writing_system: self.writing_system,
            created_at: occurred_at,
            glyphs,
            transformation: Some(transformation_id),
        };
        Ok((document, transformation))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DocumentError {
    TooManyGlyphs,
    TooManyEdits,
    InvalidEdit,
    InvalidLineage,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_copy_error_creates_inspectable_lineage() {
        let source = Document::new(
            DocumentId::new(1),
            DocumentMediumId::new(2),
            WritingSystemId::new(3),
            SimulationTime::new(4),
            vec![GlyphId::new(5), GlyphId::new(6)],
        )
        .unwrap();
        let (copy, transformation) = source
            .copy_with_edits(
                DocumentId::new(7),
                DocumentMediumId::new(8),
                SimulationTime::new(9),
                DocumentTransformationId::new(10),
                vec![GlyphEdit::Replace {
                    at: 1,
                    glyph: GlyphId::new(11),
                }],
            )
            .unwrap();
        assert_eq!(copy.parent(), Some(source.id()));
        assert_eq!(copy.glyphs(), &[GlyphId::new(5), GlyphId::new(11)]);
        assert_eq!(transformation.source, source.id());
        assert_eq!(transformation.target, copy.id());
    }

    #[test]
    fn document_has_physical_marks_but_no_objective_meaning() {
        let document = Document::new(
            DocumentId::new(1),
            DocumentMediumId::new(2),
            WritingSystemId::new(3),
            SimulationTime::new(4),
            vec![GlyphId::new(5)],
        )
        .unwrap();
        assert_eq!(document.glyphs(), &[GlyphId::new(5)]);
    }
}
