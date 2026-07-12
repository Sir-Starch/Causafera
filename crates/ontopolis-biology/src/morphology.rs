use ontopolis_types::BodySegmentId;

pub struct BodySegment {
    pub id: BodySegmentId,
    pub parent: Option<BodySegmentId>,
    pub length: f32,
}
