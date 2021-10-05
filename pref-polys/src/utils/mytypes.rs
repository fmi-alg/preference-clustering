use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::ops::{Deref, DerefMut, Index, IndexMut, Range, RangeFull, RangeInclusive};

#[derive(Debug)]
pub enum MyError {
    InvalidTrajectories,
    WrongArgumentNumber,
}

impl Display for MyError {
    fn fmt(&self, f: &mut Formatter) -> Result<(), std::fmt::Error> {
        match self {
            MyError::InvalidTrajectories => write!(f, "Invalid Trajectories"),
            MyError::WrongArgumentNumber => write!(f, "Too few arguments"),
        }
    }
}

impl std::error::Error for MyError {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MyVec<T>(pub Vec<T>);

impl<T> MyVec<T> {
    pub fn new() -> MyVec<T> {
        MyVec(Vec::new())
    }
}
impl<T> From<Vec<T>> for MyVec<T> {
    fn from(source: Vec<T>) -> Self {
        Self(source)
    }
}

impl<T> From<&[T]> for MyVec<T>
where
    T: Copy,
{
    fn from(source: &[T]) -> Self {
        Self(source.to_vec())
    }
}

impl<T> Index<u32> for MyVec<T> {
    type Output = T;

    fn index(&self, idx: u32) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<T> IndexMut<u32> for MyVec<T> {
    fn index_mut(&mut self, idx: u32) -> &mut Self::Output {
        &mut self.0[idx as usize]
    }
}

impl<T> Index<Range<u32>> for MyVec<T> {
    type Output = [T];

    fn index(&self, r: Range<u32>) -> &Self::Output {
        &self.0[r.start as usize..r.end as usize]
    }
}

impl<T> Index<RangeInclusive<u32>> for MyVec<T> {
    type Output = [T];

    fn index(&self, r: RangeInclusive<u32>) -> &Self::Output {
        let start = *r.start() as usize;
        let end = *r.end() as usize;
        &self.0[start..=end]
    }
}

impl<T> Index<usize> for MyVec<T> {
    type Output = T;

    fn index(&self, idx: usize) -> &Self::Output {
        &self.0[idx]
    }
}

impl<T> IndexMut<usize> for MyVec<T> {
    fn index_mut(&mut self, idx: usize) -> &mut Self::Output {
        &mut self.0[idx]
    }
}

impl<T> Index<Range<usize>> for MyVec<T> {
    type Output = [T];

    fn index(&self, r: Range<usize>) -> &Self::Output {
        &self.0[r.start..r.end]
    }
}

impl<T> Index<RangeFull> for MyVec<T> {
    type Output = [T];

    fn index(&self, _: RangeFull) -> &Self::Output {
        &self.0[0..self.len()]
    }
}

impl<T> Index<i32> for MyVec<T> {
    type Output = T;

    fn index(&self, idx: i32) -> &Self::Output {
        &self.0[idx as usize]
    }
}

impl<T> IndexMut<i32> for MyVec<T> {
    fn index_mut(&mut self, idx: i32) -> &mut Self::Output {
        &mut self.0[idx as usize]
    }
}

impl<T> Deref for MyVec<T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for MyVec<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
