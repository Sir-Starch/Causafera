use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU64;

use causafera_types::{TraceId, WaterAccumulator, WaterVolume};

use super::{
    HydrologyCellKey, HydrologyStateError, MAX_HYDROLOGY_FORCING_HORIZON_TICKS,
    MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK, MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK,
    MAX_HYDROLOGY_FORCING_RECORDS, MAX_HYDROLOGY_TARGETS_PER_FORCING,
    MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS,
};

/// The seventh production bootstrap stage's forcing-producer policy.
///
/// The only producer this tranche accepts. A future Climate system becomes a
/// second one by committing the same origin contract in an earlier tick and
/// phase — it needs no bootstrap ancestry — but it would need its own policy
/// ID, and none is reserved here. An unknown ID rejects rather than being
/// treated as "some producer we have not met".
pub const BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1: u64 = 1;

const fn is_registered_policy(policy: u64) -> bool {
    policy == BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1
}

/// One cell's share of a forcing record, by weight.
///
/// The weight is `NonZeroU64` because a zero-weight member would receive
/// nothing from every allocation and exist only to be skipped — a target that
/// is not targeted. Omitting it says the same thing without the ambiguity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HydrologyForcingMember {
    pub cell: HydrologyCellKey,
    pub weight: NonZeroU64,
}

impl HydrologyForcingMember {
    pub const fn new(cell: HydrologyCellKey, weight: NonZeroU64) -> Self {
        Self { cell, weight }
    }
}

/// One explicit, tick-indexed hydrologic input with committed ancestry.
///
/// This is how hydrology gets weather without implementing weather. There is no
/// season, no month, no rainfall model, and no hidden generator: a record says
/// how much water arrives at which cells at which tick, and something outside
/// hydrology had to commit an origin event before it could exist.
///
/// The three quantities are **record totals**, not per-cell amounts, and they
/// are allocated across members by weight with a largest-remainder rule, so the
/// parts sum to the whole exactly. Potential ET is a *demand*: what is actually
/// removed is bounded by available surface and soil water, and the shortfall is
/// recorded rather than treated as water that left.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyForcingRecord {
    forcing_id: u64,
    scheduled_tick: u64,
    targets: Vec<HydrologyForcingMember>,
    precipitation_volume: WaterVolume,
    potential_et_volume: WaterVolume,
    external_inflow_volume: WaterVolume,
    origin_trace: TraceId,
    producer_policy_schema: u64,
    applied_at: Option<u64>,
}

/// The complete constructor input, so nine fields of mostly numeric type
/// cannot be transposed silently at a call site.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HydrologyForcingParts {
    pub forcing_id: u64,
    pub scheduled_tick: u64,
    pub targets: Vec<HydrologyForcingMember>,
    pub precipitation_volume: WaterVolume,
    pub potential_et_volume: WaterVolume,
    pub external_inflow_volume: WaterVolume,
    pub origin_trace: TraceId,
    pub producer_policy_schema: u64,
    pub applied_at: Option<u64>,
}

impl HydrologyForcingRecord {
    pub fn new(parts: HydrologyForcingParts) -> Result<Self, HydrologyStateError> {
        if parts.targets.is_empty() {
            return Err(HydrologyStateError::EmptyForcingTargets);
        }
        if parts.targets.len() > MAX_HYDROLOGY_TARGETS_PER_FORCING {
            return Err(HydrologyStateError::ForcingTargetCountExceeded {
                count: parts.targets.len(),
                max: MAX_HYDROLOGY_TARGETS_PER_FORCING,
            });
        }
        // Sorted *and* unique, checked rather than repaired. Sorting here would
        // make two byte-different records compare equal after construction,
        // and the record's canonical bytes are what the bootstrap event's
        // result digest covers.
        for window in parts.targets.windows(2) {
            match window[0].cell.cmp(&window[1].cell) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => {
                    return Err(HydrologyStateError::DuplicateForcingMember);
                }
                std::cmp::Ordering::Greater => {
                    return Err(HydrologyStateError::UnorderedForcingMembers);
                }
            }
        }
        if !is_registered_policy(parts.producer_policy_schema) {
            return Err(HydrologyStateError::UnknownForcingPolicy(
                parts.producer_policy_schema,
            ));
        }
        // A record is applied exactly once, at the tick it was scheduled for.
        // Any other timestamp is a record that claims to have happened when it
        // could not have.
        if let Some(applied_at) = parts.applied_at
            && applied_at != parts.scheduled_tick
        {
            return Err(HydrologyStateError::ForcingAppliedOffSchedule);
        }
        Ok(Self {
            forcing_id: parts.forcing_id,
            scheduled_tick: parts.scheduled_tick,
            targets: parts.targets,
            precipitation_volume: parts.precipitation_volume,
            potential_et_volume: parts.potential_et_volume,
            external_inflow_volume: parts.external_inflow_volume,
            origin_trace: parts.origin_trace,
            producer_policy_schema: parts.producer_policy_schema,
            applied_at: parts.applied_at,
        })
    }

    pub const fn forcing_id(&self) -> u64 {
        self.forcing_id
    }

    pub const fn scheduled_tick(&self) -> u64 {
        self.scheduled_tick
    }

    pub fn targets(&self) -> &[HydrologyForcingMember] {
        &self.targets
    }

    pub const fn precipitation_volume(&self) -> WaterVolume {
        self.precipitation_volume
    }

    pub const fn potential_et_volume(&self) -> WaterVolume {
        self.potential_et_volume
    }

    pub const fn external_inflow_volume(&self) -> WaterVolume {
        self.external_inflow_volume
    }

    pub const fn origin_trace(&self) -> TraceId {
        self.origin_trace
    }

    pub const fn producer_policy_schema(&self) -> u64 {
        self.producer_policy_schema
    }

    pub const fn applied_at(&self) -> Option<u64> {
        self.applied_at
    }

    pub const fn is_applied(&self) -> bool {
        self.applied_at.is_some()
    }

    /// The canonical schedule key. Records are ordered and deduplicated on it,
    /// so overlapping records reduce in a fixed order regardless of how they
    /// were produced.
    pub const fn key(&self) -> (u64, u64) {
        (self.scheduled_tick, self.forcing_id)
    }

    /// Total member weight, the denominator of every allocation over this
    /// record. Non-zero by construction, since every weight is non-zero and
    /// there is at least one member.
    pub fn total_weight(&self) -> Result<u128, HydrologyStateError> {
        let mut total = 0_u128;
        for member in &self.targets {
            total = total
                .checked_add(u128::from(member.weight.get()))
                .ok_or(causafera_types::WaterVolumeError::Overflow)?;
        }
        Ok(total)
    }

    /// Everything this record adds to the world, if fully accepted.
    pub fn total_source(&self) -> Result<WaterAccumulator, HydrologyStateError> {
        Ok(WaterAccumulator::ZERO
            .add_volume(self.precipitation_volume)?
            .add_volume(self.external_inflow_volume)?)
    }

    /// Mark the record applied. Fails if it is already applied, or if the tick
    /// is not the one it was scheduled for.
    pub fn mark_applied(&mut self, tick: u64) -> Result<(), HydrologyStateError> {
        if self.applied_at.is_some() || tick != self.scheduled_tick {
            return Err(HydrologyStateError::ForcingAppliedOffSchedule);
        }
        self.applied_at = Some(tick);
        Ok(())
    }
}

/// The complete bounded forcing schedule, in canonical `(tick, id)` order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HydrologyForcingSchedule {
    records: Vec<HydrologyForcingRecord>,
}

impl HydrologyForcingSchedule {
    pub fn new(records: Vec<HydrologyForcingRecord>) -> Result<Self, HydrologyStateError> {
        if records.len() > MAX_HYDROLOGY_FORCING_RECORDS {
            return Err(HydrologyStateError::ForcingRecordCountExceeded {
                count: records.len(),
                max: MAX_HYDROLOGY_FORCING_RECORDS,
            });
        }

        // Ordering and uniqueness are checked, not imposed: the schedule's
        // canonical bytes are covered by the bootstrap event's result digest,
        // so silently reordering would change what the digest attests to.
        let mut members = 0_usize;
        for window in records.windows(2) {
            match window[0].key().cmp(&window[1].key()) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => {
                    return Err(HydrologyStateError::DuplicateForcingKey {
                        tick: window[1].scheduled_tick(),
                        id: window[1].forcing_id(),
                    });
                }
                std::cmp::Ordering::Greater => {
                    return Err(HydrologyStateError::UnorderedForcingSchedule);
                }
            }
        }
        for record in &records {
            // The aggregate member cap composes with the per-record one: 8_192
            // records of 4_096 targets each would satisfy both individually and
            // still be far past what the snapshot envelope can hold.
            members = members.saturating_add(record.targets().len());
            if members > MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS {
                return Err(HydrologyStateError::ForcingMemberTotalExceeded {
                    count: members,
                    max: MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS,
                });
            }
        }

        let schedule = Self { records };
        schedule.validate_origin_fan_in()?;
        Ok(schedule)
    }

    /// Enforce the two causal fan-in bounds that keep every hydrology event
    /// inside the sixteen-cause limit.
    ///
    /// A tick may draw on at most eight distinct forcing origins, and any one
    /// cell on at most six within a tick. Overlapping records are legal and
    /// expected — two producers may rain on the same ground — but the terminal
    /// conservation event cites the tick's origins and each cell's
    /// forcing-settlement event cites that cell's, so both counts are hard
    /// structural limits rather than preferences.
    fn validate_origin_fan_in(&self) -> Result<(), HydrologyStateError> {
        let mut per_tick: BTreeMap<u64, BTreeSet<TraceId>> = BTreeMap::new();
        let mut per_cell: BTreeMap<(u64, HydrologyCellKey), BTreeSet<TraceId>> = BTreeMap::new();
        for record in &self.records {
            let tick = record.scheduled_tick();
            let origins = per_tick.entry(tick).or_default();
            origins.insert(record.origin_trace());
            if origins.len() > MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK {
                return Err(HydrologyStateError::ForcingOriginsPerTickExceeded {
                    tick,
                    count: origins.len(),
                    max: MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK,
                });
            }
            for member in record.targets() {
                let cell_origins = per_cell.entry((tick, member.cell)).or_default();
                cell_origins.insert(record.origin_trace());
                if cell_origins.len() > MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK {
                    return Err(HydrologyStateError::ForcingOriginsPerCellExceeded {
                        count: cell_origins.len(),
                        max: MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK,
                    });
                }
            }
        }
        Ok(())
    }

    /// Every record is scheduled strictly after `bootstrap_tick` and no more
    /// than the horizon beyond it.
    ///
    /// Checked subtraction throughout: a schedule that wrapped would look near
    /// when it is unreachable, and a record scheduled at or before the tick
    /// bootstrap completed could never be applied at all.
    pub fn validate_bootstrap_horizon(
        &self,
        bootstrap_tick: u64,
    ) -> Result<(), HydrologyStateError> {
        for record in &self.records {
            let Some(delta) = record.scheduled_tick().checked_sub(bootstrap_tick) else {
                return Err(HydrologyStateError::ForcingAppliedOffSchedule);
            };
            if delta == 0 || delta > MAX_HYDROLOGY_FORCING_HORIZON_TICKS {
                return Err(HydrologyStateError::ForcingAppliedOffSchedule);
            }
        }
        Ok(())
    }

    pub fn records(&self) -> &[HydrologyForcingRecord] {
        &self.records
    }

    pub fn len(&self) -> usize {
        self.records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    pub fn member_count(&self) -> usize {
        self.records
            .iter()
            .map(|record| record.targets().len())
            .sum()
    }

    /// Every pending record scheduled for `tick`, in canonical order.
    pub fn pending_at(&self, tick: u64) -> impl Iterator<Item = &HydrologyForcingRecord> {
        self.records
            .iter()
            .filter(move |record| record.scheduled_tick() == tick && !record.is_applied())
    }

    /// The greatest applied `(tick, id)`, which is what the observer summary
    /// reports as the latest forcing.
    pub fn latest_applied(&self) -> Option<&HydrologyForcingRecord> {
        self.records
            .iter()
            .filter(|record| record.is_applied())
            .max_by_key(|record| record.key())
    }

    pub fn mark_applied(&mut self, key: (u64, u64), tick: u64) -> Result<(), HydrologyStateError> {
        let record = self
            .records
            .iter_mut()
            .find(|record| record.key() == key)
            .ok_or(HydrologyStateError::ForcingAppliedOffSchedule)?;
        record.mark_applied(tick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hydrology::SURFACE_CELL_COUNT;
    use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId};

    fn chunk(x: i32) -> ChartChunkCoord {
        ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(x, 0, 0))
    }

    fn cell(ordinal: u16) -> HydrologyCellKey {
        HydrologyCellKey::new(chunk(0), ordinal).expect("ordinal is in range")
    }

    fn nz(value: u64) -> NonZeroU64 {
        NonZeroU64::new(value).expect("test weights are positive")
    }

    fn member(ordinal: u16, weight: u64) -> HydrologyForcingMember {
        HydrologyForcingMember::new(cell(ordinal), nz(weight))
    }

    /// The `index`-th cell of the lattice, walking into further chunks once one
    /// chunk's `SURFACE_CELL_COUNT` ordinals run out. The per-record target cap
    /// is larger than one chunk, so a bound test needs more than one chunk to
    /// reach it with distinct cells.
    fn wide_member(index: usize) -> HydrologyForcingMember {
        let chunk_index = (index / SURFACE_CELL_COUNT) as i32;
        let ordinal = (index % SURFACE_CELL_COUNT) as u16;
        HydrologyForcingMember::new(
            HydrologyCellKey::new(chunk(chunk_index), ordinal).expect("ordinal is in range"),
            nz(1),
        )
    }

    fn parts(id: u64, tick: u64, targets: Vec<HydrologyForcingMember>) -> HydrologyForcingParts {
        HydrologyForcingParts {
            forcing_id: id,
            scheduled_tick: tick,
            targets,
            precipitation_volume: WaterVolume::new(1_000),
            potential_et_volume: WaterVolume::new(200),
            external_inflow_volume: WaterVolume::new(50),
            origin_trace: TraceId::new(7),
            producer_policy_schema: BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1,
            applied_at: None,
        }
    }

    fn record(id: u64, tick: u64) -> HydrologyForcingRecord {
        HydrologyForcingRecord::new(parts(id, tick, vec![member(0, 1), member(1, 3)]))
            .expect("test record is valid")
    }

    #[test]
    fn a_record_must_target_at_least_one_cell() {
        assert_eq!(
            HydrologyForcingRecord::new(parts(1, 5, Vec::new())),
            Err(HydrologyStateError::EmptyForcingTargets)
        );
    }

    #[test]
    fn members_must_arrive_sorted_and_unique_rather_than_being_repaired() {
        // Repairing here would make two byte-different records compare equal
        // after construction, while the bootstrap event's result digest covers
        // the bytes that went in.
        assert_eq!(
            HydrologyForcingRecord::new(parts(1, 5, vec![member(3, 1), member(1, 1)])),
            Err(HydrologyStateError::UnorderedForcingMembers)
        );
        assert_eq!(
            HydrologyForcingRecord::new(parts(1, 5, vec![member(1, 1), member(1, 2)])),
            Err(HydrologyStateError::DuplicateForcingMember)
        );
        assert!(HydrologyForcingRecord::new(parts(1, 5, vec![member(1, 1), member(3, 1)])).is_ok());
    }

    #[test]
    fn a_record_rejects_one_target_past_its_bound() {
        let at_bound = (0..MAX_HYDROLOGY_TARGETS_PER_FORCING)
            .map(wide_member)
            .collect::<Vec<_>>();
        assert!(HydrologyForcingRecord::new(parts(1, 5, at_bound.clone())).is_ok());

        // The lattice is larger than the per-record cap, so one more target is
        // still a valid cell — the rejection is the bound, not the address.
        const { assert!(MAX_HYDROLOGY_TARGETS_PER_FORCING > SURFACE_CELL_COUNT) };
        let mut over = at_bound;
        over.push(wide_member(MAX_HYDROLOGY_TARGETS_PER_FORCING));
        assert!(matches!(
            HydrologyForcingRecord::new(parts(1, 5, over)),
            Err(HydrologyStateError::ForcingTargetCountExceeded { .. })
        ));
    }

    #[test]
    fn an_unregistered_producer_policy_is_rejected() {
        let mut forged = parts(1, 5, vec![member(0, 1)]);
        forged.producer_policy_schema = 2;
        assert_eq!(
            HydrologyForcingRecord::new(forged),
            Err(HydrologyStateError::UnknownForcingPolicy(2))
        );
        assert_eq!(BOOTSTRAP_HYDROLOGY_FORCING_POLICY_V1, 1);
    }

    #[test]
    fn a_record_is_applied_exactly_once_at_its_scheduled_tick() {
        let mut applied = parts(1, 5, vec![member(0, 1)]);
        applied.applied_at = Some(4);
        assert_eq!(
            HydrologyForcingRecord::new(applied.clone()),
            Err(HydrologyStateError::ForcingAppliedOffSchedule),
            "an applied timestamp that is not the scheduled tick"
        );
        applied.applied_at = Some(5);
        let already = HydrologyForcingRecord::new(applied).unwrap();
        assert!(already.is_applied());

        let mut pending = record(1, 5);
        assert!(!pending.is_applied());
        assert_eq!(
            pending.mark_applied(4),
            Err(HydrologyStateError::ForcingAppliedOffSchedule),
            "applying off schedule"
        );
        assert!(pending.mark_applied(5).is_ok());
        assert_eq!(pending.applied_at(), Some(5));
        assert_eq!(
            pending.mark_applied(5),
            Err(HydrologyStateError::ForcingAppliedOffSchedule),
            "reapplying the same record"
        );
    }

    #[test]
    fn record_totals_are_exact_and_checked() {
        let record = record(1, 5);
        assert_eq!(record.total_weight().unwrap(), 4);
        assert_eq!(record.total_source().unwrap().get(), 1_050);
        assert_eq!(record.key(), (5, 1));

        let huge = HydrologyForcingRecord::new(HydrologyForcingParts {
            precipitation_volume: WaterVolume::MAX,
            external_inflow_volume: WaterVolume::MAX,
            ..parts(1, 5, vec![member(0, u64::MAX), member(1, u64::MAX)])
        })
        .unwrap();
        assert_eq!(huge.total_weight().unwrap(), 2 * u128::from(u64::MAX));
        assert_eq!(huge.total_source().unwrap().get(), 2 * i128::from(u64::MAX));
    }

    #[test]
    fn a_schedule_must_arrive_in_canonical_order_without_duplicates() {
        assert!(
            HydrologyForcingSchedule::new(vec![record(1, 5), record(2, 5), record(1, 6)]).is_ok()
        );
        assert_eq!(
            HydrologyForcingSchedule::new(vec![record(1, 6), record(1, 5)]),
            Err(HydrologyStateError::UnorderedForcingSchedule)
        );
        assert_eq!(
            HydrologyForcingSchedule::new(vec![record(2, 5), record(1, 5)]),
            Err(HydrologyStateError::UnorderedForcingSchedule),
            "ties break on id, ascending"
        );
        assert_eq!(
            HydrologyForcingSchedule::new(vec![record(1, 5), record(1, 5)]),
            Err(HydrologyStateError::DuplicateForcingKey { tick: 5, id: 1 })
        );
    }

    #[test]
    fn the_aggregate_member_cap_composes_with_the_per_record_one() {
        // Each record here is individually legal. The point of the aggregate
        // cap is that "legal record" times "legal record count" is not.
        let wide = |id: u64| {
            HydrologyForcingRecord::new(parts(
                id,
                5 + id,
                (0..MAX_HYDROLOGY_TARGETS_PER_FORCING)
                    .map(wide_member)
                    .collect(),
            ))
            .expect("each record is individually within its own cap")
        };
        let records_needed =
            MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS / MAX_HYDROLOGY_TARGETS_PER_FORCING;
        let at_bound = (0..records_needed as u64).map(wide).collect::<Vec<_>>();
        assert_eq!(
            HydrologyForcingSchedule::new(at_bound)
                .unwrap()
                .member_count(),
            MAX_HYDROLOGY_TOTAL_FORCING_MEMBERS
        );

        let over = (0..=records_needed as u64).map(wide).collect::<Vec<_>>();
        assert!(matches!(
            HydrologyForcingSchedule::new(over),
            Err(HydrologyStateError::ForcingMemberTotalExceeded { .. })
        ));
    }

    #[test]
    fn a_schedule_rejects_one_record_past_its_bound() {
        let one = |id: u64| {
            HydrologyForcingRecord::new(parts(0, id + 1, vec![member(0, 1)]))
                .expect("single-target record is valid")
        };
        let at_bound = (0..MAX_HYDROLOGY_FORCING_RECORDS as u64)
            .map(one)
            .collect::<Vec<_>>();
        assert!(HydrologyForcingSchedule::new(at_bound).is_ok());

        let over = (0..=MAX_HYDROLOGY_FORCING_RECORDS as u64)
            .map(one)
            .collect::<Vec<_>>();
        assert_eq!(
            HydrologyForcingSchedule::new(over),
            Err(HydrologyStateError::ForcingRecordCountExceeded {
                count: MAX_HYDROLOGY_FORCING_RECORDS + 1,
                max: MAX_HYDROLOGY_FORCING_RECORDS,
            })
        );
    }

    fn with_origin(id: u64, tick: u64, origin: u64, targets: Vec<u16>) -> HydrologyForcingRecord {
        HydrologyForcingRecord::new(HydrologyForcingParts {
            origin_trace: TraceId::new(origin),
            ..parts(
                id,
                tick,
                targets
                    .into_iter()
                    .map(|ordinal| member(ordinal, 1))
                    .collect(),
            )
        })
        .expect("test record is valid")
    }

    #[test]
    fn a_tick_may_draw_on_at_most_eight_distinct_origins() {
        // The terminal conservation event cites the tick's origins alongside
        // its aggregation root and the previous conservation trace, so this is
        // a structural limit on the committed DAG, not a preference.
        // Each record targets its own cell, so the per-cell bound (six) is not
        // what rejects here: the eight-origin tick bound is.
        let at_bound = (0..MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK as u64)
            .map(|index| with_origin(index, 5, 100 + index, vec![index as u16]))
            .collect::<Vec<_>>();
        assert!(HydrologyForcingSchedule::new(at_bound).is_ok());

        let over = (0..=MAX_HYDROLOGY_FORCING_ORIGINS_PER_TICK as u64)
            .map(|index| with_origin(index, 5, 100 + index, vec![index as u16]))
            .collect::<Vec<_>>();
        assert!(matches!(
            HydrologyForcingSchedule::new(over),
            Err(HydrologyStateError::ForcingOriginsPerTickExceeded { tick: 5, .. })
        ));

        // Many records sharing one origin are fine: it is *distinct* origins
        // that become causes.
        let shared = (0..64_u64)
            .map(|index| with_origin(index, 5, 100, vec![0]))
            .collect::<Vec<_>>();
        assert!(HydrologyForcingSchedule::new(shared).is_ok());

        // And the same origins on different ticks do not accumulate.
        let spread = (0..16_u64)
            .map(|index| with_origin(0, 5 + index, 100 + index, vec![0]))
            .collect::<Vec<_>>();
        assert!(HydrologyForcingSchedule::new(spread).is_ok());
    }

    #[test]
    fn one_cell_may_draw_on_at_most_six_distinct_origins_in_a_tick() {
        // A cell's forcing-settlement event cites its prior surface and soil
        // traces plus its origins, and must stay within sixteen causes.
        let at_bound = (0..MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK as u64)
            .map(|index| with_origin(index, 5, 100 + index, vec![9]))
            .collect::<Vec<_>>();
        assert!(HydrologyForcingSchedule::new(at_bound).is_ok());

        let over = (0..=MAX_HYDROLOGY_FORCING_ORIGINS_PER_CELL_PER_TICK as u64)
            .map(|index| with_origin(index, 5, 100 + index, vec![9]))
            .collect::<Vec<_>>();
        assert!(matches!(
            HydrologyForcingSchedule::new(over),
            Err(HydrologyStateError::ForcingOriginsPerCellExceeded { .. })
        ));

        // Seven origins on a tick spread over cells that each see at most six
        // satisfies the per-cell bound while staying inside the per-tick one.
        let spread = (0..7_u64)
            .map(|index| with_origin(index, 5, 100 + index, vec![index as u16]))
            .collect::<Vec<_>>();
        assert!(HydrologyForcingSchedule::new(spread).is_ok());
    }

    #[test]
    fn the_bootstrap_horizon_is_checked_without_wrapping() {
        let schedule = HydrologyForcingSchedule::new(vec![record(1, 5)]).unwrap();
        assert!(schedule.validate_bootstrap_horizon(4).is_ok());
        assert_eq!(
            schedule.validate_bootstrap_horizon(5),
            Err(HydrologyStateError::ForcingAppliedOffSchedule),
            "a record scheduled at the completed bootstrap tick can never apply"
        );
        // The subtraction is checked, so a record before the bootstrap tick
        // rejects rather than wrapping into an apparently distant future.
        assert_eq!(
            schedule.validate_bootstrap_horizon(6),
            Err(HydrologyStateError::ForcingAppliedOffSchedule)
        );

        let far =
            HydrologyForcingSchedule::new(vec![record(1, MAX_HYDROLOGY_FORCING_HORIZON_TICKS + 1)])
                .unwrap();
        assert!(far.validate_bootstrap_horizon(1).is_ok());
        assert_eq!(
            far.validate_bootstrap_horizon(0),
            Err(HydrologyStateError::ForcingAppliedOffSchedule),
            "one tick past the horizon"
        );
    }

    #[test]
    fn the_schedule_reports_pending_and_latest_applied_in_canonical_order() {
        let mut schedule =
            HydrologyForcingSchedule::new(vec![record(1, 5), record(2, 5), record(1, 6)]).unwrap();
        assert_eq!(schedule.len(), 3);
        assert!(!schedule.is_empty());
        assert_eq!(schedule.member_count(), 6);
        assert_eq!(schedule.latest_applied(), None);
        assert_eq!(
            schedule
                .pending_at(5)
                .map(HydrologyForcingRecord::key)
                .collect::<Vec<_>>(),
            vec![(5, 1), (5, 2)]
        );

        schedule.mark_applied((5, 1), 5).unwrap();
        assert_eq!(
            schedule
                .pending_at(5)
                .map(HydrologyForcingRecord::key)
                .collect::<Vec<_>>(),
            vec![(5, 2)],
            "an applied record is no longer pending"
        );
        schedule.mark_applied((6, 1), 6).unwrap();
        assert_eq!(
            schedule.latest_applied().map(HydrologyForcingRecord::key),
            Some((6, 1))
        );

        assert_eq!(
            schedule.mark_applied((9, 9), 9),
            Err(HydrologyStateError::ForcingAppliedOffSchedule),
            "a key that is not in the schedule"
        );
    }
}
