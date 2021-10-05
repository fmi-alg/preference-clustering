mod preference;
mod size_approx;

pub use preference::PreferenceLp;
pub use size_approx::{increase_pref_dim, lower_constraint_dimension, SizeApproxLp};

/// Given some point sets, this ConvexHullIntersection finds point which lies in
/// both convex hulls, if such a point exists
pub struct ConvexHullIntersection {
    dim: c_int,
    lp: *mut glp_prob,
    set_count: usize,
}

use glpk_sys::*;
use std::convert::TryInto;
use std::ffi::CString;
use std::os::raw::c_int;

use crate::utils::Preference;

const GLP_LO: c_int = 2; // variable with lower bound
const GLP_CV: c_int = 1; // continuous variable
const GLP_DB: c_int = 4; // double-bounded variable
const GLP_FX: c_int = 5; // fixed variable

const GLP_ON: c_int = 1; // enable something
const GLP_OFF: c_int = 0; // disable something
const GLP_MSG_OFF: c_int = 0; // no output

const GLP_OPT: c_int = 5; // solution is optimal
const GLP_FEAS: c_int = 2; // solution is feasible

impl ConvexHullIntersection {
    pub fn new(dim: usize) -> Self {
        let dim = dim.try_into().unwrap();
        let lp = unsafe {
            glp_term_out(GLP_OFF);
            let lp = glp_create_prob();
            Self::setup_result_vars(lp, dim);
            lp
        };
        Self {
            dim,
            lp,
            set_count: 0,
        }
    }

    unsafe fn setup_result_vars(lp: *mut glp_prob, dim: c_int) {
        glp_add_cols(lp, dim);
        for i in 0..dim {
            let name =
                CString::new(format!("goal_{}", i)).expect("Column name could not be created");

            glp_set_col_bnds(lp, i + 1, GLP_LO, 0.0, 0.0);
            glp_set_col_kind(lp, i + 1, GLP_CV);
            glp_set_obj_coef(lp, i + 1, 0.0);
            glp_set_col_name(lp, i + 1, name.as_ptr());
        }
    }

    pub fn add_point_set(&mut self, points: &[Preference]) {
        let point_len = points.len().try_into().unwrap();
        unsafe {
            let col = glp_add_cols(self.lp, point_len);
            for i in 0..point_len {
                let name = CString::new(format!("factor_{}_{}", self.set_count, i))
                    .expect("Column name could not be created");

                glp_set_col_bnds(self.lp, col + i, GLP_DB, 0.0, 1.0);
                glp_set_col_kind(self.lp, col + i, GLP_CV);
                glp_set_obj_coef(self.lp, col + i, 0.0);
                glp_set_col_name(self.lp, col + i, name.as_ptr());
            }
            // Constraint: sum of convex combination factors is equal to one
            let row = glp_add_rows(self.lp, 1);
            let indices: Vec<_> = std::iter::once(0).chain(col..col + point_len).collect();
            let values = vec![1.0; points.len() + 1];
            assert_eq!(indices.len(), values.len());
            glp_set_row_bnds(self.lp, row, GLP_FX, 1.0, 1.0);
            glp_set_mat_row(self.lp, row, point_len, indices.as_ptr(), values.as_ptr());

            // Constraints: in each dimension i: sum_j points[j]_i * factor_j = s_i
            // sum_j points[j]_i * factor_j - s_i = 0
            for i in 0..self.dim {
                let row = glp_add_rows(self.lp, 1);
                let indices: Vec<_> = std::iter::once(0)
                    .chain(std::iter::once(i + 1)) //s_i
                    .chain(col..col + point_len) // factor_j_i
                    .collect();
                let values: Vec<f64> = std::iter::once(0.)
                    .chain(std::iter::once(-1.)) // s_i
                    .chain(points.iter().map(|p| p[i])) // factor_j_i
                    .collect();

                assert_eq!(&indices.len(), &values.len());
                glp_set_row_bnds(self.lp, row, GLP_FX, 0.0, 0.0);
                glp_set_mat_row(
                    self.lp,
                    row,
                    point_len + 1,
                    indices.as_ptr(),
                    values.as_ptr(),
                );
            }
        }
        self.set_count += 1;
    }

    pub fn reset(&mut self) {
        self.set_count = 0;
        unsafe {
            let old = std::mem::replace(&mut self.lp, glp_create_prob());
            glp_delete_prob(old);
            Self::setup_result_vars(self.lp, self.dim);
        }
    }

    pub fn solve(&mut self) -> Option<Preference> {
        unsafe {
            let mut params = glp_smcp::default();
            glp_init_smcp(&mut params);
            params.presolve = GLP_ON;
            params.msg_lev = GLP_MSG_OFF;

            let status = glp_simplex(self.lp, &params);
            if status == 0 {
                let status = glp_get_status(self.lp);
                if !(status == GLP_OPT || status == GLP_FEAS) {
                    return None;
                }
            } else {
                return None;
            }
            let mut result = vec![0.0; self.dim as usize];
            for i in 0..self.dim {
                result[i as usize] = glp_get_col_prim(self.lp, i + 1);
            }
            Some(result.into())
        }
    }
}

impl Drop for ConvexHullIntersection {
    fn drop(&mut self) {
        unsafe { glp_delete_prob(self.lp) }
    }
}

#[test]
fn test_convex_hull_intersection() {
    let mut chi = ConvexHullIntersection::new(2);

    let triangle1 = vec![
        vec![1., 2.].into(),
        vec![3., 6.].into(),
        vec![5., 1.].into(),
    ];

    let triangle2 = vec![
        vec![8., 8.].into(),
        vec![13., 5.].into(),
        vec![13., 9.].into(),
    ];

    let triangle3 = vec![
        vec![3., 3.].into(),
        vec![12., 3.].into(),
        vec![12., 8.].into(),
    ];

    chi.add_point_set(&triangle1);
    chi.add_point_set(&triangle3);

    assert!(dbg!(chi.solve()).is_some());

    chi.add_point_set(&triangle2);
    assert!(dbg!(chi.solve()).is_none());

    chi.reset();

    chi.add_point_set(&triangle2);
    chi.add_point_set(&triangle3);
    assert!(dbg!(chi.solve()).is_some());

    chi.reset();
    chi.add_point_set(&triangle1);
    chi.add_point_set(&triangle2);

    assert!(chi.solve().is_none());
}
