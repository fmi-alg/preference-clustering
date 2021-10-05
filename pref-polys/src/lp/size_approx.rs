use crate::utils::{convert_to_f64_vec, MyVec, Preference, F64_SIZE};

use crate::ACCURACY;

use std::io::BufReader;
use std::io::{BufWriter, Read, Write};
use std::process::{Child, Command, Stdio};

use anyhow::{Context, Result};

pub struct SizeApproxLp {
    lp: Child,
    dim: usize,
}

impl SizeApproxLp {
    pub fn new(dim: usize) -> Result<SizeApproxLp> {
        let mut path = std::env::current_exe().unwrap();
        path.pop();
        path.push("lp_size_approx");

        // In case we run tests, we run from the deps directory...
        if !path.exists() {
            path.pop();
            path.pop();
            path.push("lp_size_approx");
        }

        let lp = Command::new(&path)
            .arg(dim.to_string())
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .spawn()?;

        Ok(Self { lp, dim })
    }

    pub fn buffer(dim: usize) -> Vec<u8> {
        vec![0; F64_SIZE * dim]
    }

    pub fn set_obj_fun(&mut self, coef: &[f64]) -> Result<()> {
        assert_eq!(
            self.dim - 1,
            coef.len(),
            "Tried to set objective function with wrong dimension"
        );

        let child_stdin = self.lp.stdin.as_mut().unwrap();

        let mut b = BufWriter::new(child_stdin);

        let write_buffer: Vec<_> = coef
            .iter()
            .flat_map(|c| c.to_ne_bytes().iter().copied().collect::<Vec<_>>())
            .collect();

        b.write_all(&[4u8])
            .context("writing control byte for obj function")?;
        b.write_all(&write_buffer).context("writing obj function")?;
        b.flush().context("flushing after writing obj function")?;

        Ok(())
    }

    pub fn add_constraint(&mut self, costs: &[f64]) -> Result<i32> {
        assert_eq!(
            self.dim,
            costs.len(),
            "Tried to add constraint with wrong dimension"
        );

        let mut norm_costs = vec![0.0; self.dim];

        costs.iter().zip(norm_costs.iter_mut()).for_each(|(c, n)| {
            if c.abs() < ACCURACY {
                *n = 0.0;
            } else {
                *n = *c;
            }
        });

        norm_costs = lower_constraint_dimension(&norm_costs);

        let child_stdin = self.lp.stdin.as_mut().unwrap();

        let mut w = BufWriter::new(child_stdin);

        let write_buffer: Vec<_> = norm_costs
            .iter()
            .flat_map(|c| c.to_ne_bytes().iter().copied().collect::<Vec<_>>())
            .collect();

        w.write_all(&[1u8])
            .context("Failed to write control byte")?;
        w.write_all(&write_buffer).with_context(|| {
            format!(
                "failed to write constraint {:?} with buffer of len {}",
                norm_costs,
                write_buffer.len()
            )
        })?;
        w.flush()
            .context("failed to flush after writing new constraint")?;

        let child_stdout = self.lp.stdout.as_mut().unwrap();
        let mut r = BufReader::new(child_stdout);

        let mut buf = [0u8; 4];
        r.read_exact(&mut buf)
            .context("failed to read index for new constraint")?;
        let row = i32::from_ne_bytes(buf);

        Ok(row)
    }

    pub fn reset(&mut self) -> Result<()> {
        let child_stdin = self.lp.stdin.as_mut().unwrap();

        let mut b = BufWriter::new(child_stdin);
        b.write_all(&[0u8])?;
        b.flush()?;

        Ok(())
    }

    pub fn solve(&mut self, exact: bool) -> Result<Option<Preference>> {
        let mut buffer = Self::buffer(self.dim - 1);
        let child_stdin = self.lp.stdin.as_mut().unwrap();

        let mut b = BufWriter::new(child_stdin);

        let c = if exact { 3u8 } else { 2u8 };
        b.write_all(&[c])?;
        b.flush()?;

        let child_stdout = self.lp.stdout.as_mut().unwrap();
        let mut r = BufReader::new(child_stdout);
        let mut control_byte = [0u8; 1];

        r.read_exact(&mut control_byte)?;
        match control_byte[0] {
            0 => {
                r.read_exact(&mut buffer)?;
                let mut result: Vec<_> = convert_to_f64_vec(&mut buffer);
                result.iter_mut().for_each(|r| *r = r.max(0.0));
                let sum: f64 = result.iter().sum();
                result.push(1.0 - sum);
                assert_eq!(self.dim, result.len());

                Ok(Some(result.into()))
            }
            1 => Ok(None),
            x => panic!("Unknown control byte received on main side: {}", x),
        }
    }

    pub fn non_basic_constraints(&mut self) -> Result<Vec<Vec<f64>>> {
        let child_stdin = self.lp.stdin.as_mut().unwrap();
        let mut w = BufWriter::new(child_stdin);
        w.write_all(&[5u8])
            .context("failed writing control byte for non basic constraints")?;
        w.flush()
            .context("failed flushing contol byte for non basic constraints")?;

        let child_stdout = self.lp.stdout.as_mut().unwrap();
        let mut r = BufReader::new(child_stdout);

        let mut len = [0u8; 8];

        r.read_exact(&mut len)
            .context("failed reading length of non basic constraints")?;
        let len = usize::from_ne_bytes(len);
        let mut res = Vec::with_capacity(len);

        let mut buf = [0u8; F64_SIZE];
        for c in 0..len {
            let mut constraint = Vec::with_capacity(self.dim);
            for i in 0..self.dim {
                r.read_exact(&mut buf).with_context(|| {
                    format!("failed reading {}th value of {}th constraint", i, c)
                })?;
                constraint.push(f64::from_ne_bytes(buf));
            }
            res.push(constraint);
        }
        Ok(res)
    }
}

pub fn lower_constraint_dimension(constraint: &[f64]) -> Vec<f64> {
    let mut res = Vec::from(constraint);
    let (last, rest) = res
        .split_last_mut()
        .expect("cannot lower dimension of empty constraint");

    for val in rest {
        *val -= *last;
    }
    *last = -*last;

    res
}

pub fn increase_pref_dim(p: &[f64]) -> Preference {
    let sum: f64 = p.iter().sum();
    let mut p = MyVec::from(p);
    p.push(1.0 - sum);
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::same_array;

    #[test]
    fn test_lower_dimension() {
        let constraint = [3.0, -4.0, 2.0];
        let lowered = [1.0, -6.0, -2.0];

        assert!(same_array(
            &lowered,
            &lower_constraint_dimension(&constraint)
        ));
    }

    #[test]
    fn test_getting_non_basic_constraints() {
        let mut lp = SizeApproxLp::new(3).unwrap();

        lp.set_obj_fun(&[-1.0, 1.0]).unwrap();

        lp.add_constraint(&[2.0, -1.0, -2.0]).unwrap();
        lp.add_constraint(&[-2.0, -2.0, 6.0]).unwrap();

        let sol = lp.solve(false).unwrap().unwrap();

        assert!(same_array(
            dbg!(&sol.0[..2]),
            &[0.416666666666, 0.3333333333]
        ));

        let non_basic = lp.non_basic_constraints().unwrap();
        assert_eq!(non_basic.len(), 2);
        assert!(same_array(dbg!(&non_basic[0]), &[4.0, 1.0, 2.0]));
        assert!(same_array(dbg!(&non_basic[1]), &[-8.0, -8.0, -6.0]));

        lp.set_obj_fun(&[-1.0, -1.0]).unwrap();
        let sol = lp.solve(false).unwrap().unwrap();

        assert!(same_array(dbg!(&sol.0[..2]), &[0.5, 0.0]));

        let non_basic = lp.non_basic_constraints().unwrap();
        assert_eq!(non_basic.len(), 2);

        assert!(same_array(dbg!(&non_basic[0]), &[4.0, 1.0, 2.0]));
        assert!(same_array(dbg!(&non_basic[1]), &[0.0, 1.0, 0.0]));
    }
}
