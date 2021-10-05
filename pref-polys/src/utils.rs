use rand::{
    distributions::{Distribution, Uniform},
    prelude::SliceRandom,
    RngCore,
};

mod bitset;
pub mod io;
mod matrix;
pub mod metrics;
mod mytypes;

pub use bitset::{BitSet, BitSetFns, BitSetIter, GrowingBitSet, GrowingBitSetIter};
pub use matrix::SquareMatrix;
pub use mytypes::{MyError, MyVec};

pub type Preference = MyVec<f64>;
pub type Costs = MyVec<f64>;

pub const F64_SIZE: usize = std::mem::size_of::<f64>();

#[macro_export]
macro_rules! float_eq {
    ($lhs:expr, $rhs:expr) => {
        approx::abs_diff_eq!($lhs, $rhs, epsilon = $crate::ACCURACY)
    };
}

pub fn equal_weights(dim: usize) -> MyVec<f64> {
    vec![1.0 / dim as f64; dim].into()
}

pub fn randomized_preference(rng: &mut dyn RngCore, dim: usize) -> MyVec<f64> {
    let mut result = vec![0.0; dim];
    let (last, elements) = result.split_last_mut().unwrap();
    let mut rest = 1.0;
    for r in elements.iter_mut() {
        let pref_dist = Uniform::new(0.0, rest);
        let a: f64 = pref_dist.sample(rng);
        *r = a;
        rest -= a;
    }
    *last = rest;

    result.shuffle(rng);
    result.into()
}

pub fn same_array(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(a, b)| float_eq!(a, b))
}

pub fn convert_to_f64_vec(buffer: &mut [u8]) -> Vec<f64> {
    let mut byte_buffer = [0u8; F64_SIZE];
    buffer
        .chunks_exact(F64_SIZE)
        .map(|slice| {
            byte_buffer.copy_from_slice(slice);
            f64::from_ne_bytes(byte_buffer)
        })
        .collect()
}
