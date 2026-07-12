use ontopolis_types::SimulationTime;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use crate::phases::Phase;

/// Key material for seeding a deterministic random stream.
///
/// Each stream is seeded independently so that consuming random values
/// from one stream never affects another. This is the foundation of
/// deterministic parallelism: every system gets its own stream keyed by
/// `(world_seed, time, phase, system_id)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamKey {
    pub world_seed: u64,
    pub time: SimulationTime,
    pub phase: Phase,
    pub system_id: u64,
}

impl StreamKey {
    /// Produce a 32-byte seed suitable for [`StdRng::from_seed`].
    ///
    /// The layout is deterministic and packed as follows:
    /// - bytes 0..8:   `world_seed`
    /// - bytes 8..16:  `time.raw()`
    /// - byte 16:      `phase` discriminant
    /// - bytes 17..25: `system_id`
    /// - bytes 25..32: reserved (zero)
    pub const fn to_seed(self) -> [u8; 32] {
        let world_seed = self.world_seed.to_le_bytes();
        let time = self.time.raw().to_le_bytes();
        let system_id = self.system_id.to_le_bytes();
        let phase = self.phase as u8;

        [
            world_seed[0],
            world_seed[1],
            world_seed[2],
            world_seed[3],
            world_seed[4],
            world_seed[5],
            world_seed[6],
            world_seed[7],
            time[0],
            time[1],
            time[2],
            time[3],
            time[4],
            time[5],
            time[6],
            time[7],
            phase,
            system_id[0],
            system_id[1],
            system_id[2],
            system_id[3],
            system_id[4],
            system_id[5],
            system_id[6],
            system_id[7],
            0,
            0,
            0,
            0,
            0,
            0,
            0,
        ]
    }
}

/// Deterministic random number stream.
///
/// Created from a [`StreamKey`] and backed by a ChaCha12 RNG. Every
/// instance with the same key produces the exact same sequence of values.
/// Streams are cheap to create, so each system receives a fresh stream per
/// tick rather than maintaining long-lived state.
#[derive(Clone)]
pub struct RandomStream {
    rng: StdRng,
}

impl RandomStream {
    /// Create a stream from the given key.
    pub fn from_key(key: StreamKey) -> Self {
        Self {
            rng: StdRng::from_seed(key.to_seed()),
        }
    }

    /// Return the next `u64`.
    pub fn next_u64(&mut self) -> u64 {
        self.rng.r#gen()
    }

    /// Return the next `f64` in `[0.0, 1.0)`.
    pub fn next_f64(&mut self) -> f64 {
        self.rng.r#gen()
    }

    /// Return a value uniformly distributed in the given range.
    pub fn gen_range<T, R>(&mut self, range: R) -> T
    where
        T: rand::distributions::uniform::SampleUniform,
        R: rand::distributions::uniform::SampleRange<T>,
    {
        self.rng.gen_range(range)
    }

    /// Shuffle a slice in-place deterministically.
    pub fn shuffle<T>(&mut self, slice: &mut [T]) {
        slice.shuffle(&mut self.rng);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_key_same_sequence() {
        let key = StreamKey {
            world_seed: 12345,
            time: SimulationTime::new(7),
            phase: Phase::Physics,
            system_id: 42,
        };
        let mut a = RandomStream::from_key(key);
        let mut b = RandomStream::from_key(key);

        for _ in 0..100 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_keys_different_sequences() {
        let key_a = StreamKey {
            world_seed: 12345,
            time: SimulationTime::new(7),
            phase: Phase::Physics,
            system_id: 42,
        };
        let key_b = StreamKey {
            world_seed: 12345,
            time: SimulationTime::new(7),
            phase: Phase::Physics,
            system_id: 43,
        };
        let mut a = RandomStream::from_key(key_a);
        let mut b = RandomStream::from_key(key_b);

        let first_a = a.next_u64();
        let first_b = b.next_u64();
        assert_ne!(first_a, first_b);
    }

    #[test]
    fn shuffle_is_deterministic() {
        let key = StreamKey {
            world_seed: 999,
            time: SimulationTime::new(0),
            phase: Phase::Cognition,
            system_id: 1,
        };
        let mut a = [1, 2, 3, 4, 5];
        let mut b = [1, 2, 3, 4, 5];

        RandomStream::from_key(key).shuffle(&mut a);
        RandomStream::from_key(key).shuffle(&mut b);

        assert_eq!(a, b);
    }
}
