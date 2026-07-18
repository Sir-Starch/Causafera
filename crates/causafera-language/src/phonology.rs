use causafera_types::{LanguageId, PhonologicalUnitId};

pub const MAX_PHONOLOGICAL_UNITS: usize = 24;
pub const MAX_FORM_UNITS: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PhonemeInventory {
    language_id: LanguageId,
    units: Vec<PhonologicalUnitId>,
    onset_count: u8,
    nucleus_count: u8,
    coda_count: u8,
}

impl PhonemeInventory {
    pub fn new(
        language_id: LanguageId,
        mut units: Vec<PhonologicalUnitId>,
        onset_count: u8,
        nucleus_count: u8,
        coda_count: u8,
    ) -> Result<Self, PhonologyError> {
        units.sort_unstable();
        units.dedup();
        let total = usize::from(onset_count) + usize::from(nucleus_count) + usize::from(coda_count);
        if units.is_empty() || units.len() > MAX_PHONOLOGICAL_UNITS || total != units.len() {
            return Err(PhonologyError::InvalidInventory);
        }
        if onset_count == 0 || nucleus_count == 0 {
            return Err(PhonologyError::InvalidInventory);
        }
        Ok(Self {
            language_id,
            units,
            onset_count,
            nucleus_count,
            coda_count,
        })
    }

    pub fn language_id(&self) -> LanguageId {
        self.language_id
    }

    pub fn units(&self) -> &[PhonologicalUnitId] {
        &self.units
    }

    pub fn accepts(&self, form: &PhonologicalForm) -> bool {
        let len = form.units.len();
        if !(2..=3).contains(&len) {
            return false;
        }
        let onset_end = usize::from(self.onset_count);
        let nucleus_end = onset_end + usize::from(self.nucleus_count);
        let onset = self.units[..onset_end]
            .binary_search(&form.units[0])
            .is_ok();
        let nucleus = self.units[onset_end..nucleus_end]
            .binary_search(&form.units[1])
            .is_ok();
        let coda = len == 2
            || self.units[nucleus_end..]
                .binary_search(&form.units[2])
                .is_ok();
        onset && nucleus && coda
    }

    pub fn generate(&self, key: u64) -> PhonologicalForm {
        let onset_end = usize::from(self.onset_count);
        let nucleus_end = onset_end + usize::from(self.nucleus_count);
        let onset = self.units[(key as usize) % onset_end];
        let nucleus = self.units
            [onset_end + ((key.rotate_left(17) as usize) % usize::from(self.nucleus_count))];
        let mut units = vec![onset, nucleus];
        if self.coda_count > 0 && key & 1 == 1 {
            units.push(
                self.units
                    [nucleus_end + ((key.rotate_left(31) as usize) % usize::from(self.coda_count))],
            );
        }
        PhonologicalForm { units }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct PhonologicalForm {
    units: Vec<PhonologicalUnitId>,
}

impl PhonologicalForm {
    pub fn new(units: Vec<PhonologicalUnitId>) -> Result<Self, PhonologyError> {
        if units.is_empty() || units.len() > MAX_FORM_UNITS {
            return Err(PhonologyError::InvalidForm);
        }
        Ok(Self { units })
    }

    pub fn units(&self) -> &[PhonologicalUnitId] {
        &self.units
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhonologyError {
    InvalidInventory,
    InvalidForm,
}

pub(crate) fn mix64(mut value: u64) -> u64 {
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^ (value >> 31)
}
