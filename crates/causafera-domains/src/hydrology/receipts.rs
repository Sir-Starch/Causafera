use causafera_types::WaterAccumulator;

use super::{HydrologyBucket, HydrologyError, HydrologyTransferReceipt};

/// The exact water budget of one tick.
///
/// Every term is `i128`, and `residual` must be exactly zero before any event
/// is committed. Not "close to zero" and not "within a tolerance": the whole
/// point of a fixed-point carrier is that the ledger closes, and a tolerance
/// would be a place for a silent sink to live.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HydrologyConservationReceipt {
    tick: u64,
    batch_sequence: u64,
    surface_before: i128,
    soil_before: i128,
    groundwater_before: i128,
    conveyance_before: i128,
    surface_after: i128,
    soil_after: i128,
    groundwater_after: i128,
    conveyance_after: i128,
    accepted_precipitation: i128,
    accepted_external_inflow: i128,
    accepted_evapotranspiration: i128,
    boundary_exports: i128,
    residual: i128,
}

/// Complete constructor input for a conservation receipt.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HydrologyConservationParts {
    pub tick: u64,
    pub batch_sequence: u64,
    pub surface_before: i128,
    pub soil_before: i128,
    pub groundwater_before: i128,
    pub conveyance_before: i128,
    pub surface_after: i128,
    pub soil_after: i128,
    pub groundwater_after: i128,
    pub conveyance_after: i128,
    pub accepted_precipitation: i128,
    pub accepted_external_inflow: i128,
    pub accepted_evapotranspiration: i128,
    pub boundary_exports: i128,
}

impl HydrologyConservationReceipt {
    /// Build the receipt and compute its residual. Construction succeeds for a
    /// nonzero residual — reporting the number is the receipt's job. Refusing
    /// to commit on it is [`Self::require_balanced`]'s.
    pub fn new(parts: HydrologyConservationParts) -> Result<Self, HydrologyError> {
        let storage_before = WaterAccumulator::ZERO
            .add(parts.surface_before)?
            .add(parts.soil_before)?
            .add(parts.groundwater_before)?
            .add(parts.conveyance_before)?;
        let storage_after = WaterAccumulator::ZERO
            .add(parts.surface_after)?
            .add(parts.soil_after)?
            .add(parts.groundwater_after)?
            .add(parts.conveyance_after)?;
        let sources = WaterAccumulator::ZERO
            .add(parts.accepted_precipitation)?
            .add(parts.accepted_external_inflow)?;
        let sinks = WaterAccumulator::ZERO
            .add(parts.accepted_evapotranspiration)?
            .add(parts.boundary_exports)?;
        let residual = storage_before
            .add(sources.get())?
            .sub(storage_after.get())?
            .sub(sinks.get())?
            .get();
        Ok(Self {
            tick: parts.tick,
            batch_sequence: parts.batch_sequence,
            surface_before: parts.surface_before,
            soil_before: parts.soil_before,
            groundwater_before: parts.groundwater_before,
            conveyance_before: parts.conveyance_before,
            surface_after: parts.surface_after,
            soil_after: parts.soil_after,
            groundwater_after: parts.groundwater_after,
            conveyance_after: parts.conveyance_after,
            accepted_precipitation: parts.accepted_precipitation,
            accepted_external_inflow: parts.accepted_external_inflow,
            accepted_evapotranspiration: parts.accepted_evapotranspiration,
            boundary_exports: parts.boundary_exports,
            residual,
        })
    }

    pub const fn tick(&self) -> u64 {
        self.tick
    }

    pub const fn batch_sequence(&self) -> u64 {
        self.batch_sequence
    }

    pub const fn residual(&self) -> i128 {
        self.residual
    }

    pub const fn surface_before(&self) -> i128 {
        self.surface_before
    }

    pub const fn soil_before(&self) -> i128 {
        self.soil_before
    }

    pub const fn groundwater_before(&self) -> i128 {
        self.groundwater_before
    }

    pub const fn conveyance_before(&self) -> i128 {
        self.conveyance_before
    }

    pub const fn surface_after(&self) -> i128 {
        self.surface_after
    }

    pub const fn soil_after(&self) -> i128 {
        self.soil_after
    }

    pub const fn groundwater_after(&self) -> i128 {
        self.groundwater_after
    }

    pub const fn conveyance_after(&self) -> i128 {
        self.conveyance_after
    }

    pub const fn accepted_precipitation(&self) -> i128 {
        self.accepted_precipitation
    }

    pub const fn accepted_external_inflow(&self) -> i128 {
        self.accepted_external_inflow
    }

    pub const fn accepted_evapotranspiration(&self) -> i128 {
        self.accepted_evapotranspiration
    }

    pub const fn boundary_exports(&self) -> i128 {
        self.boundary_exports
    }

    pub fn storage_before(&self) -> Result<i128, HydrologyError> {
        Ok(WaterAccumulator::ZERO
            .add(self.surface_before)?
            .add(self.soil_before)?
            .add(self.groundwater_before)?
            .add(self.conveyance_before)?
            .get())
    }

    pub fn storage_after(&self) -> Result<i128, HydrologyError> {
        Ok(WaterAccumulator::ZERO
            .add(self.surface_after)?
            .add(self.soil_after)?
            .add(self.groundwater_after)?
            .add(self.conveyance_after)?
            .get())
    }

    pub fn sources(&self) -> Result<i128, HydrologyError> {
        Ok(WaterAccumulator::ZERO
            .add(self.accepted_precipitation)?
            .add(self.accepted_external_inflow)?
            .get())
    }

    pub fn sinks(&self) -> Result<i128, HydrologyError> {
        Ok(WaterAccumulator::ZERO
            .add(self.accepted_evapotranspiration)?
            .add(self.boundary_exports)?
            .get())
    }

    /// The gate every hydrology tick passes before a single event commits.
    pub const fn require_balanced(&self) -> Result<(), HydrologyError> {
        if self.residual == 0 {
            Ok(())
        } else {
            Err(HydrologyError::ConservationResidual {
                residual: self.residual,
            })
        }
    }
}

/// Source, sink, and internal-transfer totals recomputed from receipts alone.
///
/// This exists so the ledger can be checked against something other than
/// itself. A conservation receipt built from the same running totals the solver
/// accumulated would agree with the solver by construction and prove nothing;
/// folding the independently-recorded per-transfer receipts back up is a second
/// derivation of the same numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HydrologyReceiptTotals {
    pub accepted_precipitation: i128,
    pub accepted_external_inflow: i128,
    pub accepted_evapotranspiration: i128,
    pub boundary_exports: i128,
    pub internal_transfers: i128,
}

impl HydrologyReceiptTotals {
    pub fn from_receipts(receipts: &[HydrologyTransferReceipt]) -> Result<Self, HydrologyError> {
        use super::process;

        let mut totals = Self::default();
        for receipt in receipts {
            let accepted = receipt.accepted().as_i128();
            match receipt.process_kind() {
                process::PRECIPITATION => {
                    totals.accepted_precipitation =
                        WaterAccumulator::new(totals.accepted_precipitation)
                            .add(accepted)?
                            .get();
                }
                process::EXTERNAL_INFLOW => {
                    totals.accepted_external_inflow =
                        WaterAccumulator::new(totals.accepted_external_inflow)
                            .add(accepted)?
                            .get();
                }
                process::EVAPOTRANSPIRATION_SURFACE | process::EVAPOTRANSPIRATION_SOIL => {
                    totals.accepted_evapotranspiration =
                        WaterAccumulator::new(totals.accepted_evapotranspiration)
                            .add(accepted)?
                            .get();
                }
                process::SURFACE_BOUNDARY_EXPORT | process::GROUNDWATER_BOUNDARY_EXPORT => {
                    totals.boundary_exports = WaterAccumulator::new(totals.boundary_exports)
                        .add(accepted)?
                        .get();
                }
                _ => {
                    totals.internal_transfers = WaterAccumulator::new(totals.internal_transfers)
                        .add(accepted)?
                        .get();
                }
            }
        }
        Ok(totals)
    }

    /// Cross-check a conservation receipt against receipts recomputed from
    /// scratch. Import validation and tests both use this rather than trusting
    /// the aggregate literals a snapshot happens to carry.
    pub fn agrees_with(&self, receipt: &HydrologyConservationReceipt) -> bool {
        self.accepted_precipitation == receipt.accepted_precipitation()
            && self.accepted_external_inflow == receipt.accepted_external_inflow()
            && self.accepted_evapotranspiration == receipt.accepted_evapotranspiration()
            && self.boundary_exports == receipt.boundary_exports()
    }
}

/// Every receipt whose source and target are real storage must move the same
/// amount out of one and into the other.
///
/// A transfer that subtracted a different amount than it added would be a
/// source or a sink wearing an internal transfer's process kind, and the
/// terminal residual alone could not tell you which cell it happened in.
pub fn validate_paired_transfers(
    receipts: &[HydrologyTransferReceipt],
) -> Result<(), HydrologyError> {
    for receipt in receipts {
        if !receipt.is_internal_transfer() {
            continue;
        }
        let withdrawn = WaterAccumulator::ZERO
            .add_volume(receipt.source_before())?
            .sub_volume(receipt.source_after())?
            .get();
        let deposited = WaterAccumulator::ZERO
            .add_volume(receipt.target_after())?
            .sub_volume(receipt.target_before())?
            .get();
        if withdrawn != receipt.accepted().as_i128() || deposited != receipt.accepted().as_i128() {
            return Err(HydrologyError::ReceiptInconsistent);
        }
    }
    Ok(())
}

/// A sink receipt must remove exactly what it accepted, and a source receipt
/// must deposit exactly what it accepted.
pub fn validate_boundary_transfers(
    receipts: &[HydrologyTransferReceipt],
) -> Result<(), HydrologyError> {
    for receipt in receipts {
        match (receipt.source_bucket(), receipt.target_bucket()) {
            (HydrologyBucket::External, HydrologyBucket::External) => {
                return Err(HydrologyError::ReceiptInconsistent);
            }
            (HydrologyBucket::External, _) => {
                let deposited = WaterAccumulator::ZERO
                    .add_volume(receipt.target_after())?
                    .sub_volume(receipt.target_before())?
                    .get();
                if deposited != receipt.accepted().as_i128() {
                    return Err(HydrologyError::ReceiptInconsistent);
                }
            }
            (_, HydrologyBucket::External) => {
                let withdrawn = WaterAccumulator::ZERO
                    .add_volume(receipt.source_before())?
                    .sub_volume(receipt.source_after())?
                    .get();
                if withdrawn != receipt.accepted().as_i128() {
                    return Err(HydrologyError::ReceiptInconsistent);
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn balanced() -> HydrologyConservationParts {
        HydrologyConservationParts {
            tick: 5,
            batch_sequence: 2,
            surface_before: 100,
            soil_before: 50,
            groundwater_before: 25,
            conveyance_before: 10,
            surface_after: 120,
            soil_after: 45,
            groundwater_after: 25,
            conveyance_after: 10,
            accepted_precipitation: 30,
            accepted_external_inflow: 0,
            accepted_evapotranspiration: 15,
            boundary_exports: 0,
        }
    }

    #[test]
    fn a_balanced_tick_has_exactly_zero_residual() {
        // before 185 + sources 30 - after 200 - sinks 15 == 0
        let receipt = HydrologyConservationReceipt::new(balanced()).unwrap();
        assert_eq!(receipt.storage_before().unwrap(), 185);
        assert_eq!(receipt.storage_after().unwrap(), 200);
        assert_eq!(receipt.sources().unwrap(), 30);
        assert_eq!(receipt.sinks().unwrap(), 15);
        assert_eq!(receipt.residual(), 0);
        assert!(receipt.require_balanced().is_ok());
    }

    #[test]
    fn a_single_unit_of_drift_is_reported_and_refused() {
        // One cubic millimetre out of 185 000 000 would round away under any
        // tolerance. There is no tolerance.
        for drift in [-1_i128, 1] {
            let mut parts = balanced();
            parts.surface_after += drift;
            let receipt = HydrologyConservationReceipt::new(parts).unwrap();
            assert_eq!(receipt.residual(), -drift);
            assert_eq!(
                receipt.require_balanced(),
                Err(HydrologyError::ConservationResidual { residual: -drift })
            );
        }
    }

    #[test]
    fn the_residual_is_signed_so_both_loss_and_gain_are_reportable() {
        let mut lost = balanced();
        lost.surface_after -= 7;
        assert_eq!(
            HydrologyConservationReceipt::new(lost).unwrap().residual(),
            7,
            "water that vanished leaves a positive residual"
        );

        let mut gained = balanced();
        gained.surface_after += 7;
        assert_eq!(
            HydrologyConservationReceipt::new(gained)
                .unwrap()
                .residual(),
            -7,
            "water that appeared leaves a negative one"
        );
    }

    #[test]
    fn whole_world_totals_stay_exact_far_past_a_u64() {
        // 131 072 cells of three buckets each, all full, is well past `u64`.
        // The ledger runs in `i128` precisely so this closes rather than wraps.
        let full = i128::from(u64::MAX) * 131_072 * 3;
        let receipt = HydrologyConservationReceipt::new(HydrologyConservationParts {
            surface_before: full,
            surface_after: full,
            ..HydrologyConservationParts::default()
        })
        .unwrap();
        assert_eq!(receipt.residual(), 0);
        assert!(receipt.require_balanced().is_ok());
    }

    #[test]
    fn receipt_totals_and_conservation_totals_are_two_derivations_of_one_number() {
        use super::super::{
            HydrologyBucket, HydrologyTransferParts, HydrologyTransferReceipt, process,
        };
        use causafera_geography::{HydrologyCarrierKey, HydrologyCellKey};
        use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, WaterVolume};

        let cell = HydrologyCellKey::new(
            ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0)),
            0,
        )
        .unwrap();
        let carrier = HydrologyCarrierKey::Cell(cell);
        let receipt = |kind: u32, source: HydrologyBucket, target: HydrologyBucket, amount: u64| {
            HydrologyTransferReceipt::new(HydrologyTransferParts {
                batch_sequence: 2,
                tick: 5,
                process_kind: kind,
                source: carrier,
                source_bucket: source,
                target: carrier,
                target_bucket: target,
                requested: WaterVolume::new(amount),
                accepted: WaterVolume::new(amount),
                source_before: WaterVolume::new(amount),
                source_after: WaterVolume::ZERO,
                target_before: WaterVolume::ZERO,
                target_after: WaterVolume::new(amount),
                causal_parents: Vec::new(),
                forcing_origin: None,
                transfer_event: None,
                storage_event: None,
            })
            .unwrap()
        };

        let receipts = vec![
            receipt(
                process::PRECIPITATION,
                HydrologyBucket::External,
                HydrologyBucket::Surface,
                30,
            ),
            receipt(
                process::EVAPOTRANSPIRATION_SOIL,
                HydrologyBucket::Soil,
                HydrologyBucket::External,
                15,
            ),
            receipt(
                process::INFILTRATION,
                HydrologyBucket::Surface,
                HydrologyBucket::Soil,
                10,
            ),
        ];

        let totals = HydrologyReceiptTotals::from_receipts(&receipts).unwrap();
        assert_eq!(totals.accepted_precipitation, 30);
        assert_eq!(totals.accepted_evapotranspiration, 15);
        assert_eq!(totals.internal_transfers, 10);
        assert!(totals.agrees_with(&HydrologyConservationReceipt::new(balanced()).unwrap()));

        assert!(validate_paired_transfers(&receipts).is_ok());
        assert!(validate_boundary_transfers(&receipts).is_ok());

        // A totals set that disagrees with the aggregate literals is caught,
        // which is the whole reason the second derivation exists.
        let mut wrong = totals;
        wrong.accepted_precipitation += 1;
        assert!(!wrong.agrees_with(&HydrologyConservationReceipt::new(balanced()).unwrap()));
    }

    #[test]
    fn an_unpaired_internal_transfer_is_caught_before_the_terminal_residual() {
        use super::super::{HydrologyBucket, HydrologyTransferParts, process};
        use causafera_geography::{HydrologyCarrierKey, HydrologyCellKey};
        use causafera_types::{ChartChunkCoord, ChunkCoord, SpatialChartId, WaterVolume};

        let cell = HydrologyCellKey::new(
            ChartChunkCoord::new(SpatialChartId::new(1), ChunkCoord::new(0, 0, 0)),
            0,
        )
        .unwrap();
        let carrier = HydrologyCarrierKey::Cell(cell);
        // Takes 10 from the surface, deposits only 9 in the soil. The terminal
        // residual would notice one unit missing somewhere in the world; this
        // says which receipt lost it.
        let broken = HydrologyTransferReceipt::new(HydrologyTransferParts {
            batch_sequence: 1,
            tick: 1,
            process_kind: process::INFILTRATION,
            source: carrier,
            source_bucket: HydrologyBucket::Surface,
            target: carrier,
            target_bucket: HydrologyBucket::Soil,
            requested: WaterVolume::new(10),
            accepted: WaterVolume::new(10),
            source_before: WaterVolume::new(10),
            source_after: WaterVolume::ZERO,
            target_before: WaterVolume::ZERO,
            target_after: WaterVolume::new(9),
            causal_parents: Vec::new(),
            forcing_origin: None,
            transfer_event: None,
            storage_event: None,
        })
        .unwrap();
        assert_eq!(
            validate_paired_transfers(&[broken]),
            Err(HydrologyError::ReceiptInconsistent)
        );
    }
}
