use std::ops::{Index, IndexMut, Mul};

use super::MyVec;

#[derive(Clone, Debug)]
pub struct SquareMatrix {
    dim: usize,
    data: Vec<f64>,
}

impl SquareMatrix {
    pub fn new(dim: usize) -> Self {
        let data = vec![0.0; dim * dim];

        Self { dim, data }
    }
    // pub fn with_data(dim: usize, data: Vec<f64>) -> Self {
    //     assert_eq!(dim * dim, data.len(), "Matrix data with wrong size");
    //     Self { dim, data }
    // }
}

impl Index<usize> for SquareMatrix {
    type Output = [f64];

    fn index(&self, index: usize) -> &Self::Output {
        let start = index * self.dim;
        let end = (index + 1) * self.dim;
        &self.data[start..end]
    }
}

impl IndexMut<usize> for SquareMatrix {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let start = index * self.dim;
        let end = (index + 1) * self.dim;
        &mut self.data[start..end]
    }
}

impl Mul<&[f64]> for &SquareMatrix {
    type Output = MyVec<f64>;

    fn mul(self, rhs: &[f64]) -> Self::Output {
        debug_assert_eq!(
            self.dim,
            rhs.len(),
            "Try to multply with vec of wrong length"
        );
        let mut result = vec![0.0; self.dim];
        for i in 0..self.dim {
            result[i] = self[i].iter().zip(rhs).map(|(m, v)| m * v).sum();
        }
        result.into()
    }
}

#[test]
fn test_setting_values_in_matrix() {
    use crate::float_eq;

    let dim = 3;
    let mut mat = SquareMatrix::new(dim);
    let row_1 = &mut mat[1];
    assert_eq!(row_1.len(), dim);
    row_1[2] = 3.4;

    assert!(float_eq!(mat[1][2], 3.4));
}

#[test]
fn test_matix_vector_multiplication() {
    let dim = 3;
    let mut mat = SquareMatrix::new(dim);
    for d in 0..dim {
        mat[d][d] = d as f64;
    }
    let vec = vec![3., 2., 1.];

    let res = &mat * &vec;

    let expected = [0., 2., 2.];
    assert!(super::same_array(dbg!(&expected), dbg!(&res)))
}
