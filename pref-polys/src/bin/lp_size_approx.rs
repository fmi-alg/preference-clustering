use anyhow::Context;
use pref_polys::{
    float_eq,
    lp::SizeApproxLp,
    utils::{convert_to_f64_vec, MyVec, F64_SIZE},
};

#[cfg(feature = "debug")]
use lazy_static::lazy_static;
#[cfg(feature = "debug")]
use parking_lot::Mutex;

use glpk_sys::*;

use std::env::args;
use std::ffi::CString;
use std::io::{BufReader, BufWriter, Read, Write};
use std::os::raw::c_int;

const GLP_MAX: c_int = 2; // maximisation
const GLP_LO: c_int = 2; // variable with lower bound
const GLP_CV: c_int = 1; // continuous variable

// const GLP_DB: c_int = 4; // double-bounded variable
// const GLP_UP: c_int = 3; // variable with upper bound
// const GLP_FX: c_int = 5; // fixed variable

const GLP_ON: c_int = 1; // enable something
const GLP_OFF: c_int = 0; // disable something
const GLP_MSG_OFF: c_int = 0; // no output
const GLP_OPT: c_int = 5; // solution is optimal
const GLP_FEAS: c_int = 2; // solution is feasible

// const GLP_DUALP: c_int = 2; // use dual; if it fails, use primal

const GLP_BS: c_int = 1; // basic variable
const GLP_NL: c_int = 2; // non-basic variable on lower bound
const GLP_NU: c_int = 3; // non-basic variable on upper bound
const GLP_NF: c_int = 4; // non-basic free (unbounded) variable
const GLP_NS: c_int = 5; // non-basic fixed variable

struct Lp {
    lp: *mut glp_prob,
    counter: usize,
    dim: c_int,
}

impl Lp {
    fn new(counter: usize, dim: c_int) -> Lp {
        let lp = unsafe {
            let lp = glp_create_prob();
            glp_set_obj_dir(lp, GLP_MAX);
            Self::init_variables(lp, dim);
            lp
        };
        let mut lp = Self { lp, counter, dim };
        unsafe {
            lp.add_sum_of_alpha_eq_one();
        }
        lp
    }

    unsafe fn init_variables(lp: *mut glp_prob, dim: c_int) {
        let dim = dim - 1;
        glp_add_cols(lp, dim);
        for i in 0..dim {
            let name =
                CString::new(format!("alpha_{}", i)).expect("Column name could not be created");
            glp_set_col_bnds(lp, i + 1, GLP_LO, 0.0, 1.0);
            glp_set_col_kind(lp, i + 1, GLP_CV);
            glp_set_col_name(lp, i + 1, name.as_ptr());
        }
    }

    unsafe fn add_sum_of_alpha_eq_one(&mut self) {
        let dim = self.dim - 1;
        let row = glp_add_rows(self.lp, 1);
        let indices: Vec<_> = (0..=dim).collect();
        let values = vec![-1.0; dim as usize + 1];

        glp_set_row_bnds(self.lp, row, GLP_LO, -1.0, 1.0);
        glp_set_mat_row(self.lp, row, dim, indices.as_ptr(), values.as_ptr());
    }

    fn add_constraint(&mut self, coeff: &[f64]) -> i32 {
        if coeff.len() != self.dim as usize {
            panic!(
                "got wrong number of coefficients ({} instead of {})",
                coeff.len(),
                self.dim
            );
        }
        let (last, rest) = coeff.split_last().unwrap();
        let dim = self.dim - 1;
        unsafe {
            let row = glp_add_rows(self.lp, 1);
            // leading 0 + indices for alpha cols
            let indices: Vec<_> = (0..=dim).collect();

            // leading 0 + values for alpha cols
            let values: Vec<_> = std::iter::once(0.0).chain(rest.iter().copied()).collect();

            // 0 <= cost(alpha, p_alpha) - cost(alpha, p_trajectory)
            //
            glp_set_row_bnds(self.lp, row, GLP_LO, *last, 0.0);
            glp_set_mat_row(self.lp, row, dim, indices.as_ptr(), values.as_ptr());
            row
        }
    }

    fn set_obj_fun(&mut self, coeff: &[f64]) {
        if coeff.len() != (self.dim - 1) as usize {
            panic!(
                "got wrong number of coefficients ({} instead of {})",
                coeff.len(),
                self.dim
            );
        }
        unsafe {
            for (i, c) in coeff.iter().enumerate() {
                glp_set_obj_coef(self.lp, i as c_int + 1, *c);
            }
        }
    }

    fn solve(&mut self, exact: bool) -> Result<MyVec<f64>, LpError> {
        unsafe {
            let mut params = glp_smcp::default();
            glp_init_smcp(&mut params);
            params.presolve = GLP_ON;
            params.msg_lev = GLP_MSG_OFF;
            // params.meth = GLP_DUALP;

            #[cfg(feature = "debug")]
            || -> () {
                let filename =
                    CString::new(format!("/tmp/lps/size-approx-{}.lp", self.counter)).unwrap();
                let file_stat = glp_write_lp(self.lp, std::ptr::null(), filename.as_ptr());
                if file_stat != 0 {
                    panic!(
                        "could not write file into {}",
                        filename.into_string().unwrap()
                    );
                }
            }();
            self.counter += 1;

            let status = if exact {
                glp_exact(self.lp, &params)
            } else {
                glp_simplex(self.lp, &params)
            };
            if status == 0 {
                let status = glp_get_status(self.lp);
                if !(status == GLP_OPT || status == GLP_FEAS) {
                    return Err(LpError::Infeasible);
                }
            } else {
                return Err(LpError::Infeasible);
            }
            let dim = self.dim - 1;
            let mut result = vec![0.0; dim as usize];
            for i in 0..dim {
                result[i as usize] = glp_get_col_prim(self.lp, i + 1);
            }
            Ok(result.into())
        }
    }

    fn non_basic_constraints(&self) -> Vec<Vec<f64>> {
        let mut res = Vec::new();
        unsafe {
            for i in 1..=glp_get_num_rows(self.lp) {
                match glp_get_row_stat(self.lp, i) {
                    GLP_BS => {} // skip basic rows
                    GLP_NL | GLP_NU | GLP_NF | GLP_NS => {
                        let mut indices = vec![0i32; self.dim as usize];
                        let mut values = vec![0.0; self.dim as usize];
                        let len =
                            glp_get_mat_row(self.lp, i, indices.as_mut_ptr(), values.as_mut_ptr());
                        assert!(len < self.dim);

                        err(&format!("indices: {:?}", indices));
                        err(&format!("values: {:?}", values));
                        let mut constraint = vec![0.0; self.dim as usize];
                        for (&j, &v) in indices.iter().zip(&values) {
                            let j = j as usize;
                            if j == 0 {
                                continue;
                            }
                            constraint[j - 1] = v;
                        }
                        let last = constraint.last_mut().unwrap();
                        assert!(float_eq!(*last, 0.0));
                        *last = glp_get_row_lb(self.lp, i);
                        err(&format!("constraint: {:?}", constraint));
                        res.push(constraint);
                    }

                    x => panic!("unknown row stat: {}", x),
                }
            }
            for j in 1..=glp_get_num_cols(self.lp) {
                match glp_get_col_stat(self.lp, j) {
                    GLP_BS => {}
                    GLP_NL => {
                        let lower = glp_get_col_lb(self.lp, j);
                        let mut constraint = vec![0.0; self.dim as usize];
                        constraint[(j - 1) as usize] = 1.0;
                        let last = constraint.last_mut().unwrap();
                        assert!(float_eq!(*last, 0.0));
                        *last = lower;
                        res.push(constraint);
                    }
                    x => panic!("unknown col stat: {}", x),
                }
            }

            assert_eq!(res.len(), self.dim as usize - 1);
            res
        }
    }
}

impl Drop for Lp {
    fn drop(&mut self) {
        unsafe {
            glp_delete_prob(self.lp);
        }
    }
}

#[cfg(feature = "debug")]
lazy_static! {
    static ref ERR: Mutex<BufWriter<std::fs::File>> = Mutex::new(BufWriter::new(
        std::fs::File::create("/tmp/lperror").unwrap()
    ));
}

pub fn err(_s: &str) {
    #[cfg(feature = "debug")]
    {
        let mut e = ERR.lock();
        writeln!(e, "{}", _s).unwrap();
        e.flush().unwrap();
    }
}

fn main() -> anyhow::Result<()> {
    unsafe {
        glp_term_out(GLP_OFF);
    }

    let stdin = std::io::stdin();
    let stdin = stdin.lock();
    let mut reader = BufReader::new(stdin);

    let stdout = std::io::stdout();
    let stdout = stdout.lock();
    let mut writer = BufWriter::new(stdout);

    let mut control_byte = [0u8; 1];
    let dim = args()
        .nth(1)
        .expect("need dimension as argument")
        .parse()
        .expect("Could not parse dimension from argument");

    let mut buffer = SizeApproxLp::buffer(dim as usize);
    let mut lp = Lp::new(0, dim);
    loop {
        if reader.read_exact(&mut control_byte).is_err() {
            return Ok(());
        }

        match control_byte[0] {
            0 => lp = Lp::new(lp.counter, dim),
            1 => {
                err(&format!("reading constraint, buffer size {}", buffer.len()));
                reader.read_exact(&mut buffer)?;

                let values: Vec<_> = convert_to_f64_vec(&mut buffer);
                err(&format!("received {:?}", values));

                let row = lp.add_constraint(&values);
                err(&format!("writing row id: {:?}", row));

                writer.write_all(&row.to_ne_bytes())?;
                writer.flush()?;
            }
            x if x == 2 || x == 3 => {
                match lp.solve(x == 3) {
                    Ok(results) => {
                        let mut output = SizeApproxLp::buffer((dim - 1) as usize);

                        results
                            .iter()
                            .zip(output.chunks_exact_mut(F64_SIZE))
                            .for_each(|(f, slice)| {
                                slice.copy_from_slice(&f.to_ne_bytes());
                            });

                        control_byte[0] = 0;
                        writer.write_all(&control_byte)?;
                        writer.write_all(&output)?;
                    }
                    Err(LpError::Infeasible) => {
                        control_byte[0] = 1;
                        writer.write_all(&control_byte)?;
                    }
                }
                writer.flush()?;
            }
            4 => {
                let mut buffer = SizeApproxLp::buffer((dim - 1) as usize);
                reader.read_exact(&mut buffer).with_context(|| {
                    format!("reading obj function into buffer of size {}", buffer.len())
                })?;

                let values: Vec<_> = convert_to_f64_vec(&mut buffer);

                lp.set_obj_fun(&values);
            }
            5 => {
                let non_basic_constraints = lp.non_basic_constraints();

                let constraint_count = non_basic_constraints.len();
                writer.write_all(&constraint_count.to_ne_bytes())?;
                let mut buffer = vec![0u8; F64_SIZE * constraint_count * dim as usize];
                non_basic_constraints
                    .iter()
                    .flatten()
                    .zip(buffer.chunks_exact_mut(F64_SIZE))
                    .for_each(|(f, slice)| slice.copy_from_slice(&f.to_ne_bytes()));
                writer.write_all(&buffer)?;
                writer.flush()?;
            }
            other => panic!("Unknown control byte received on lp side: {}", other),
        }
    }
}

#[derive(Debug, Clone)]
enum LpError {
    Infeasible,
}
