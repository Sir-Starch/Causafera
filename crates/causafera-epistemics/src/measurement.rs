use causafera_types::{
    CalibrationId, MeasurementId, PracticeId, QuantitySchemaId, SimulationTime, UnitId,
};

pub const MAX_CALIBRATIONS: usize = 64;
pub const MAX_CALIBRATION_DEPTH: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnitDefinition {
    pub id: UnitId,
    pub quantity: QuantitySchemaId,
    pub scale_numerator: u32,
    pub scale_denominator: u32,
    pub resolution: u64,
}

impl UnitDefinition {
    pub fn new(
        id: UnitId,
        quantity: QuantitySchemaId,
        scale_numerator: u32,
        scale_denominator: u32,
        resolution: u64,
    ) -> Result<Self, MeasurementError> {
        if scale_numerator == 0 || scale_denominator == 0 || resolution == 0 {
            return Err(MeasurementError::InvalidUnit);
        }
        Ok(Self {
            id,
            quantity,
            scale_numerator,
            scale_denominator,
            resolution,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Calibration {
    pub id: CalibrationId,
    pub parent: Option<CalibrationId>,
    pub unit: UnitId,
    pub systematic_bias: i64,
    pub uncertainty: u64,
    pub calibrated_at: SimulationTime,
    pub procedure: PracticeId,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CalibrationRegistry {
    calibrations: Vec<Calibration>,
}

impl CalibrationRegistry {
    pub fn register(&mut self, calibration: Calibration) -> Result<(), MeasurementError> {
        if self.calibrations.len() == MAX_CALIBRATIONS {
            return Err(MeasurementError::RegistryFull);
        }
        if self
            .calibrations
            .iter()
            .any(|item| item.id == calibration.id)
        {
            return Err(MeasurementError::DuplicateCalibration);
        }
        if let Some(parent) = calibration.parent {
            let Some(parent_calibration) = self.get(parent) else {
                return Err(MeasurementError::MissingParent);
            };
            if parent_calibration.unit != calibration.unit {
                return Err(MeasurementError::UnitMismatch);
            }
            if self.depth(parent) >= MAX_CALIBRATION_DEPTH {
                return Err(MeasurementError::CalibrationTooDeep);
            }
        }
        self.calibrations.push(calibration);
        self.calibrations.sort_unstable_by_key(|item| item.id);
        Ok(())
    }

    pub fn get(&self, id: CalibrationId) -> Option<&Calibration> {
        self.calibrations
            .binary_search_by_key(&id, |item| item.id)
            .ok()
            .map(|index| &self.calibrations[index])
    }

    pub fn ancestry(&self, id: CalibrationId) -> Vec<CalibrationId> {
        let mut ancestry = Vec::with_capacity(MAX_CALIBRATION_DEPTH);
        let mut cursor = Some(id);
        while let Some(current) = cursor {
            let Some(calibration) = self.get(current) else {
                break;
            };
            ancestry.push(current);
            cursor = calibration.parent;
        }
        ancestry
    }

    fn depth(&self, id: CalibrationId) -> usize {
        self.ancestry(id).len()
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AccessibleObservation {
    pub quantity: QuantitySchemaId,
    pub observed_value: i64,
    pub access_uncertainty: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Measurement {
    pub id: MeasurementId,
    pub quantity: QuantitySchemaId,
    pub unit: UnitId,
    pub value: i64,
    pub resolution: u64,
    pub uncertainty: u64,
    pub calibration: CalibrationId,
    pub procedure: PracticeId,
    pub measured_at: SimulationTime,
}

pub fn measure(
    id: MeasurementId,
    unit: UnitDefinition,
    calibration: &Calibration,
    observation: AccessibleObservation,
    measured_at: SimulationTime,
) -> Result<Measurement, MeasurementError> {
    if unit.quantity != observation.quantity || unit.id != calibration.unit {
        return Err(MeasurementError::UnitMismatch);
    }
    let biased = observation
        .observed_value
        .saturating_add(calibration.systematic_bias);
    let scaled = i128::from(biased).saturating_mul(i128::from(unit.scale_denominator))
        / i128::from(unit.scale_numerator);
    let bounded = scaled.clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
    let quantum = i64::try_from(unit.resolution).unwrap_or(i64::MAX);
    let value = bounded.div_euclid(quantum).saturating_mul(quantum);
    Ok(Measurement {
        id,
        quantity: observation.quantity,
        unit: unit.id,
        value,
        resolution: unit.resolution,
        uncertainty: observation
            .access_uncertainty
            .saturating_add(calibration.uncertainty)
            .saturating_add(unit.resolution / 2),
        calibration: calibration.id,
        procedure: calibration.procedure,
        measured_at,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementError {
    InvalidUnit,
    RegistryFull,
    DuplicateCalibration,
    MissingParent,
    UnitMismatch,
    CalibrationTooDeep,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn socially_defined_unit_quantizes_without_truth_access() {
        let unit = UnitDefinition::new(UnitId::new(1), QuantitySchemaId::new(2), 2, 1, 5).unwrap();
        let calibration = Calibration {
            id: CalibrationId::new(3),
            parent: None,
            unit: unit.id,
            systematic_bias: 2,
            uncertainty: 3,
            calibrated_at: SimulationTime::new(1),
            procedure: PracticeId::new(4),
        };
        let observation = AccessibleObservation {
            quantity: unit.quantity,
            observed_value: 21,
            access_uncertainty: 4,
        };
        let first = measure(
            MeasurementId::new(5),
            unit,
            &calibration,
            observation,
            SimulationTime::new(8),
        )
        .unwrap();
        let second = measure(
            MeasurementId::new(5),
            unit,
            &calibration,
            observation,
            SimulationTime::new(8),
        )
        .unwrap();
        assert_eq!(first, second);
        assert_eq!(first.value, 10);
        assert_eq!(first.uncertainty, 9);
    }

    #[test]
    fn calibration_ancestry_is_validated_and_ordered() {
        let mut registry = CalibrationRegistry::default();
        for (id, parent) in [(1, None), (2, Some(1)), (3, Some(2))] {
            registry
                .register(Calibration {
                    id: CalibrationId::new(id),
                    parent: parent.map(CalibrationId::new),
                    unit: UnitId::new(9),
                    systematic_bias: 0,
                    uncertainty: 1,
                    calibrated_at: SimulationTime::new(id),
                    procedure: PracticeId::new(7),
                })
                .unwrap();
        }
        assert_eq!(
            registry.ancestry(CalibrationId::new(3)),
            vec![
                CalibrationId::new(3),
                CalibrationId::new(2),
                CalibrationId::new(1)
            ]
        );
    }
}
