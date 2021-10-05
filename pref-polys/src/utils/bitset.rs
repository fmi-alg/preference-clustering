use std::convert::TryInto;

// pub type BitSet = u128;
#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct BitSet(u128);

#[derive(Debug, Clone)]
pub struct BitSetIter {
    bits: BitSet,
    counter: usize,
}

impl BitSetIter {
    pub fn new(bits: BitSet) -> Self {
        Self { bits, counter: 0 }
    }
}

impl Iterator for BitSetIter {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        let mut res = None;
        while self.bits.0 > 0 {
            if self.bits.0 % 2 == 1 {
                res = Some(self.counter);
            }
            self.bits.0 /= 2;
            self.counter += 1;

            if res.is_some() {
                break;
            }
        }
        res
    }
}

pub trait BitSetFns<'a>
where
    &'a Self: IntoIterator + 'a,
{
    type Element;
    fn new() -> Self;
    fn add(&mut self, i: Self::Element);
    fn contains(&self, i: Self::Element) -> bool;
    fn remove(&mut self, i: Self::Element);
    fn is_empty(&self) -> bool;
    fn union(&self, other: &Self) -> Self;
    fn intersect(&self, other: &Self) -> Self;
    fn print(&self);
    fn iter(&'a self) -> <&'a Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl BitSetFns<'_> for BitSet {
    type Element = u8;
    fn add(&mut self, i: Self::Element) {
        self.0 |= 1 << i;
    }

    fn contains(&self, i: Self::Element) -> bool {
        self.0 & (1 << i) != 0
    }

    fn remove(&mut self, i: Self::Element) {
        self.0 &= u128::MAX - (1 << i)
    }

    fn is_empty(&self) -> bool {
        self.0 == 0
    }

    fn union(&self, other: &Self) -> Self {
        Self(self.0 | other.0)
    }

    fn intersect(&self, other: &Self) -> Self {
        Self(self.0 & other.0)
    }

    fn new() -> Self {
        Self(0)
    }

    fn print(&self) {
        for x in self.iter() {
            print!("{} ", x);
        }
        println!();
    }
}

impl From<u128> for BitSet {
    fn from(source: u128) -> Self {
        Self(source)
    }
}

impl std::ops::Sub<BitSet> for BitSet {
    type Output = BitSet;

    fn sub(self, rhs: BitSet) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl std::ops::Add<BitSet> for BitSet {
    type Output = BitSet;

    fn add(self, rhs: BitSet) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl<'a> IntoIterator for &'a BitSet {
    type Item = usize;

    type IntoIter = BitSetIter;

    fn into_iter(self) -> Self::IntoIter {
        BitSetIter::new(*self)
    }
}

#[test]
fn test_bit_set_functions() {
    let mut set = BitSet::new();

    set.add(124);
    set.add(1);
    set.add(15);

    assert!(set.contains(1));
    assert!(set.contains(15));
    assert!(set.contains(124));

    set.remove(15);
    assert!(!set.contains(15));

    let mut iter = set.iter();

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(124));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_removing_from_empty_set() {
    let mut set = BitSet::new();

    set.remove(15);

    assert!(set.is_empty())
}

#[test]
fn test_bitset_intersection() {
    let mut set1 = BitSet::new();
    let mut set2 = BitSet::new();

    set1.add(3);
    set1.add(5);
    set2.add(5);
    set2.add(6);

    let inter = set1.intersect(&set2);

    let mut iter = inter.iter();

    assert_eq!(iter.next(), Some(5));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_bitset_union() {
    let mut set1 = BitSet::new();
    let mut set2 = BitSet::new();

    set1.add(3);
    set1.add(5);
    set2.add(5);
    set2.add(6);

    let union = set1.union(&set2);

    let mut iter = union.iter();

    assert_eq!(iter.next(), Some(3));
    assert_eq!(iter.next(), Some(5));
    assert_eq!(iter.next(), Some(6));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_bit_set_iter() {
    let mut cell = BitSet::new();
    cell.add(124);
    cell.add(1);
    cell.add(3);
    cell.add(5);
    cell.add(7);
    cell.add(9);
    cell.add(11);
    cell.add(13);
    let mut iter = cell.iter();

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(3));
    assert_eq!(iter.next(), Some(5));
    assert_eq!(iter.next(), Some(7));
    assert_eq!(iter.next(), Some(9));
    assert_eq!(iter.next(), Some(11));
    assert_eq!(iter.next(), Some(13));
    assert_eq!(iter.next(), Some(124));
    assert_eq!(iter.next(), None);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrowingBitSet(Vec<BitSet>);

impl GrowingBitSet {
    fn element_into_index_and_val(i: u32) -> (usize, u8) {
        let index = (i / 128)
            .try_into()
            .expect("could not convert into right index type");
        let val = (i % 128)
            .try_into()
            .expect("could not convert into correct element type");
        (index, val)
    }
    fn index_and_val_into_usize(index: usize, val: u8) -> usize {
        128 * index + val as usize
    }
    fn grow_if_necessary(&mut self, index: usize) {
        if !self.has_index(index) {
            self.0.resize(index + 1, BitSet::new())
        }
    }

    fn has_index(&self, index: usize) -> bool {
        self.0.len() > index
    }
}

impl BitSetFns<'_> for GrowingBitSet {
    type Element = u32;
    fn new() -> Self {
        Self(vec![BitSet::new(), BitSet::new()])
    }

    fn add(&mut self, i: Self::Element) {
        let (index, val) = Self::element_into_index_and_val(i);
        self.grow_if_necessary(index);
        self.0[index].add(val);
    }

    fn contains(&self, i: Self::Element) -> bool {
        let (index, val) = Self::element_into_index_and_val(i);
        if !self.has_index(index) {
            return false;
        }
        self.0[index].contains(val)
    }

    fn remove(&mut self, i: Self::Element) {
        let (index, val) = Self::element_into_index_and_val(i);
        if !self.has_index(index) {
            return;
        }
        self.0[index].remove(val)
    }

    fn is_empty(&self) -> bool {
        self.0.iter().all(|s| s.is_empty())
    }

    fn union(&self, other: &Self) -> Self {
        Self(
            self.0
                .iter()
                .zip(&other.0)
                .map(|(s, o)| s.union(o))
                .collect(),
        )
    }

    fn intersect(&self, other: &Self) -> Self {
        Self(
            self.0
                .iter()
                .zip(&other.0)
                .map(|(s, o)| s.intersect(o))
                .collect(),
        )
    }

    fn print(&self) {
        for x in self.iter() {
            print!("{} ", x);
        }
        println!();
    }
}

impl<'a> IntoIterator for &'a GrowingBitSet {
    type Item = usize;

    type IntoIter = GrowingBitSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        GrowingBitSetIter {
            set: self,
            index: 0,
            iter: self.0[0].iter(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct GrowingBitSetIter<'a> {
    set: &'a GrowingBitSet,
    index: usize,
    iter: BitSetIter,
}

impl<'a> Iterator for GrowingBitSetIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(val) = self.iter.next() {
                println!("looking at {}", val);
                return Some(GrowingBitSet::index_and_val_into_usize(
                    self.index,
                    val.try_into().unwrap(),
                ));
            }
            println!("increasing index: {}", self.index);
            self.index += 1;
            if self.set.has_index(self.index) {
                println!("setting new iter");
                self.iter = self.set.0[self.index].iter();
            } else {
                return None;
            }
        }
    }
}

#[test]
fn test_growing_bit_set() {
    let mut set = GrowingBitSet::new();

    set.add(124);
    set.add(1);
    set.add(15);
    set.add(199);

    assert!(set.contains(1));
    assert!(set.contains(15));
    assert!(set.contains(124));
    assert!(set.contains(199));

    set.remove(15);
    assert!(!set.contains(15));

    set.add(987);
    assert!(set.contains(987));
    assert!(!set.contains(986));

    let mut iter = set.into_iter();

    assert_eq!(iter.next(), Some(1));
    assert_eq!(iter.next(), Some(124));
    assert_eq!(iter.next(), Some(199));
    assert_eq!(iter.next(), Some(987));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_removing_from_empty_growing_set() {
    let mut set = GrowingBitSet::new();

    set.remove(15);
    set.remove(9876);

    assert!(set.is_empty())
}

#[test]
fn test_growing_bitset_intersection() {
    let mut set1 = GrowingBitSet::new();
    let mut set2 = GrowingBitSet::new();

    set1.add(3);
    set1.add(244);
    set1.add(995);
    set1.add(5);

    set2.add(5);
    set2.add(995);
    set2.add(6);
    set2.add(522);

    let inter = set1.intersect(&set2);

    let mut iter = inter.iter();

    assert_eq!(iter.next(), Some(5));
    assert_eq!(iter.next(), Some(995));
    assert_eq!(iter.next(), None);
}

#[test]
fn test_growing_bitset_union() {
    let mut set1 = GrowingBitSet::new();
    let mut set2 = GrowingBitSet::new();

    set1.add(3);
    set1.add(244);
    set1.add(995);
    set1.add(5);

    set2.add(5);
    set2.add(995);
    set2.add(6);
    set2.add(522);

    let union = set1.union(&set2);

    let mut iter = union.iter();

    assert_eq!(iter.next(), Some(3));
    assert_eq!(iter.next(), Some(5));
    assert_eq!(iter.next(), Some(6));
    assert_eq!(iter.next(), Some(244));
    assert_eq!(iter.next(), Some(522));
    assert_eq!(iter.next(), Some(995));
    assert_eq!(iter.next(), None);
}
