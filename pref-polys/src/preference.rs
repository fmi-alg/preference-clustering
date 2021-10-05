use crate::{
    geom::{angle, center_point, intersection, orientation_test, Orientation},
    lp::{increase_pref_dim, lower_constraint_dimension, ConvexHullIntersection},
    utils::metrics::{SimpleTime, YesNoTime},
    utils::{BitSet, BitSetFns},
};
use std::{cmp::Ordering, collections::VecDeque, convert::TryInto};

use anyhow::Result;
use itertools::Itertools;
use lazy_static::lazy_static;
use metered::metered;
use ordered_float::OrderedFloat;

use crate::{
    float_eq,
    graph::path::{costs_by_alpha, Path},
    lp::{PreferenceLp, SizeApproxLp},
    utils::{same_array, Costs, MyVec, Preference, SquareMatrix},
    ACCURACY,
};
use crate::{
    graph::dijkstra::{find_path, Dijkstra},
    utils::equal_weights,
};

/// `SetPreferences` calculates preferences for which a subsets of a
/// given set of paths is optimal. It caches constraints which where generated
/// through prior run for performance.
pub struct SetPreferences<'d, 'p> {
    pub path_set: &'p [Path],
    dijkstra: Dijkstra<'d>,
    lp: PreferenceLp,
    constraints: Vec<Vec<Costs>>,
    inner_points: Vec<Vec<Preference>>,
    do_dijkstra: BitSet,
}
lazy_static! {
    static ref SET_PREF_METRICS: SetPrefMetrics = Default::default();
    static ref APPROX_METRICS: ApproxMetrics = Default::default();
}

pub struct TimeReports;

impl TimeReports {
    pub fn subset_preference() {
        println!("Subset preference report:",);
        println!("{}", SET_PREF_METRICS.subset_preference.yes_no_time);
        println!("----------");
    }

    pub fn subset_preference_lp_only() {
        println!("Subset Preference (LP only)/ No Filter report:",);
        println!("{}", SET_PREF_METRICS.subset_preference_lp_only.yes_no_time);
        println!("----------");
    }

    pub fn yes_filter() {
        println!("Yes Filter report:");
        println!("{}", SET_PREF_METRICS.yes_filter.yes_no_time);
        println!("----------");
    }
    pub fn approximate_pref_spaces() {
        println!("Preference Space Approximation report:");
        println!("{}", APPROX_METRICS.constrained_approx.simple_time);
        println!("----------");
    }
}

#[metered(registry = SetPrefMetrics, registry_expr=SET_PREF_METRICS)]
impl<'d, 'p> SetPreferences<'d, 'p> {
    /// Creates a new `SetPreferences`. The `path_set` is fixed for each instance.
    pub fn new(dijkstra: Dijkstra<'d>, path_set: &'p [Path]) -> Result<Self> {
        let constraints = vec![Vec::new(); path_set.len()];

        let lp = PreferenceLp::new(dijkstra.graph.dim.try_into().unwrap())?;
        let mut do_dijkstra = BitSet::new();
        for i in 0..(path_set.len() as u8) {
            do_dijkstra.add(i);
        }
        let inner_points = vec![Vec::new(); path_set.len()];

        Ok(SetPreferences {
            path_set,
            dijkstra,
            lp,
            constraints,
            inner_points,
            do_dijkstra,
        })
    }

    /// Calculates a preference for a subset of paths are optimal. The items
    /// yielded by `subset_indices` are interpreted as indices into the path set
    /// of the instance. Any iterator which yields &usize can be used here. For example
    /// ```no_run
    ///  # let graph = pref_covers::graph::Graph::new(vec![],vec![]);
    ///  # let dijkstra =  pref_covers::graph::dijkstra::Dijkstra::new(&graph);
    ///  # let paths = Vec::new();
    ///  # use pref_covers::preference::SetPreferences;
    ///  let mut set_pref = SetPreferences::new(dijkstra, &paths).unwrap();
    ///  let pref = set_pref.subset_preference([3,6,9].iter().copied());
    /// ```
    #[measure(YesNoTime)]
    pub fn subset_preference(
        &mut self,
        subset_indices: impl Iterator<Item = usize> + Clone,
    ) -> Result<Option<Preference>> {
        let pref = self.yes_filter(subset_indices.clone());

        if pref.is_some() {
            return Ok(pref);
        }

        let mut pref_finder =
            PrefFinder::new(&mut self.lp, self.dijkstra.graph.dim.try_into().unwrap());

        let mut all_paths = std::mem::take(&mut self.path_set);
        let mut constraints = std::mem::take(&mut self.constraints);
        let do_dijkstra = self.do_dijkstra;

        let only_with_dijkstra = subset_indices
            .clone()
            .filter(|&i| do_dijkstra.contains(i as u8));

        let path_iter = only_with_dijkstra
            .clone()
            .map(|i| -> &Path { &all_paths[i] });

        let constr_iter = subset_indices.flat_map(|i| -> &[Costs] { &constraints[i] });

        let res = pref_finder.constrained_multi_path_preference(
            &mut self.dijkstra,
            path_iter,
            constr_iter,
        );

        let (pref, constr) = match res {
            Ok((pref, constr)) => (pref, constr),
            Err(err) => {
                std::mem::swap(&mut self.path_set, &mut all_paths);
                std::mem::swap(&mut self.constraints, &mut constraints);
                return Err(err);
            }
        };

        only_with_dijkstra
            .zip(constr.into_iter())
            .for_each(|(i, c)| {
                constraints[i].extend(c);
                constraints[i].sort_by(|a, b| {
                    a.iter().zip(b.iter()).fold(Ordering::Equal, |acc, (a, b)| {
                        acc.then(a.partial_cmp(b).unwrap())
                    })
                });
                constraints[i].dedup_by(|a, b| same_array(a, b));
            });

        std::mem::swap(&mut self.path_set, &mut all_paths);
        std::mem::swap(&mut self.constraints, &mut constraints);

        Ok(pref)
    }

    #[measure(YesNoTime)]
    fn yes_filter(&mut self, subset_indices: impl Iterator<Item = usize>) -> Option<Preference> {
        let mut inner_filter_applicable = true;
        let mut chi = ConvexHullIntersection::new(self.path_set[0].total_dimension_costs.len());
        for i in subset_indices {
            if self.inner_points[i].is_empty() {
                inner_filter_applicable = false;
                break;
            }
            chi.add_point_set(&self.inner_points[i]);
        }
        if inner_filter_applicable {
            return chi.solve();
        }
        None
    }

    pub fn approximate_pref_spaces(
        &mut self,
        directions: impl Iterator<Item = Preference> + Clone + Send,
    ) -> Result<Vec<SizeApproximation>> {
        let dim = self.dijkstra.graph.dim.try_into().unwrap();

        let thread_count = num_cpus::get().min(self.path_set.len());
        let item_per_thread = self.path_set.len() / thread_count;

        let thread_res = crossbeam::scope(|scope| {
            let chunks = self.path_set.chunks(item_per_thread);
            let mut handles = Vec::new();

            for chunk in chunks.into_iter() {
                let directions = directions.clone();
                let dijkstra = self.dijkstra.clone();
                let handle = scope.spawn(move |_| {
                    let mut dijkstra = dijkstra;
                    let mut lp =
                        SizeApproxLp::new(dim).expect("could no create size approximation LP");
                    let mut approximator = PrefSizeApproximator::new(&mut lp, &mut dijkstra);

                    chunk
                        .iter()
                        .map(|p| approximator.approx(p, directions.clone()).unwrap())
                        .collect::<Vec<_>>()
                });
                handles.push(handle);
            }

            let mut inner_res = Vec::with_capacity(self.path_set.len());
            for handle in handles {
                inner_res.extend(handle.join().expect("Cannot join thread"))
            }
            inner_res
        });

        Ok(thread_res.expect("Threading for approximation failed"))
    }
    pub fn constraints(&self, index: usize) -> &[Costs] {
        &self.constraints[index]
    }
    pub fn do_dijkstra(&mut self, index: usize, active: bool) {
        if active && !self.do_dijkstra.contains(index as u8) {
            self.do_dijkstra.add(index as u8);
        } else if !active && self.do_dijkstra.contains(index as u8) {
            self.do_dijkstra.remove(index as u8);
        }
    }

    pub fn exact_pref_spaces(&mut self) -> Vec<Vec<ApproxPoint>> {
        let dim = self.dijkstra.graph.dim;
        assert_eq!(
            dim, 3,
            "exact preferences spaces only implemented for dimension 3"
        );
        let mut result = Vec::new();
        for p in self.path_set {
            result.push(self.exact_pref_space(p));
        }
        result
    }

    fn exact_pref_space(&mut self, p: &Path) -> Vec<ApproxPoint> {
        let mut constraints = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![-1.0, -1.0, -1.0],
        ];
        let mut candidates = constraints
            .iter()
            .tuple_combinations()
            .map(|(a, b)| ApproxPoint {
                point: increase_pref_dim(&intersection(a, b).unwrap()),
                constraints: vec![a.clone(), b.clone()],
            })
            .collect::<VecDeque<_>>();
        let mut points = Vec::new();
        while !candidates.is_empty() {
            let candidate = candidates.pop_front().unwrap();
            let path_check = find_constraint_for_path(p, &mut self.dijkstra, &candidate.point);
            if float_eq!(path_check.dif, 0.0) {
                points.push(candidate);
                continue;
            }
            let constraint = lower_constraint_dimension(&path_check.constraint);
            candidates.extend(
                candidate
                    .constraints
                    .iter()
                    .filter_map(|c| {
                        let point = intersection(c, &constraint);
                        point.map(|point| ApproxPoint {
                            point,
                            constraints: vec![c.clone(), constraint.clone()],
                        })
                    })
                    .filter(|a| {
                        0.0 <= a.point[0]
                            && a.point[0] <= 1.0
                            && 0.0 <= a.point[1]
                            && a.point[1] <= 1.0
                    })
                    .filter(|p| {
                        constraints
                            .iter()
                            .all(|c| orientation_test(c, &p.point) == Orientation::Inside)
                    })
                    .map(|mut p| {
                        p.point = increase_pref_dim(&p.point);
                        p
                    }),
            );
            constraints.push(constraint);
        }

        let points_only: Vec<_> = points.iter().map(|p| p.point.clone()).collect();
        let center = center_point(&points_only);
        points.sort_by_cached_key(|p| OrderedFloat(angle(&p.point, &center)));
        points.dedup_by(|a, b| same_array(&a.point, &b.point));
        points
    }

    #[measure(YesNoTime)]
    pub fn subset_preference_lp_only(
        &mut self,
        subset_indices: impl Iterator<Item = usize> + Clone,
    ) -> Result<Option<Preference>> {
        let pref = self.yes_filter(subset_indices.clone());
        if pref.is_some() {
            return Ok(pref);
        }
        let mut pref_finder =
            PrefFinder::new(&mut self.lp, self.dijkstra.graph.dim.try_into().unwrap());

        let mut constraints = std::mem::take(&mut self.constraints);

        let res = pref_finder.constrained_multi_path_preference_lp_only(
            subset_indices.flat_map(|i| -> &[Costs] { &constraints[i] }),
        );
        std::mem::swap(&mut self.constraints, &mut constraints);
        res
    }
}

pub struct PrefFinder<'b> {
    lp: &'b mut PreferenceLp,
    dim: usize,
}

impl<'b> PrefFinder<'b> {
    pub fn new(lp: &'b mut PreferenceLp, dim: usize) -> Self {
        lp.reset().expect("Could not reset lp");
        PrefFinder { lp, dim }
    }

    pub fn path_preference(
        &mut self,
        dijkstra: &mut Dijkstra,
        path: &Path,
    ) -> Result<Option<Preference>> {
        self.multi_path_preference(dijkstra, &[path.clone()])
    }

    pub fn multi_path_preference(
        &mut self,
        dijkstra: &mut Dijkstra,
        paths: &[Path],
    ) -> Result<Option<Preference>> {
        let (pref, _) =
            self.constrained_multi_path_preference(dijkstra, paths.iter(), std::iter::empty())?;
        Ok(pref)
    }

    pub fn constrained_multi_path_preference<'p>(
        &mut self,
        dijkstra: &mut Dijkstra,
        paths: impl Iterator<Item = &'p Path> + Clone,
        constraints: impl Iterator<Item = &'p Costs>,
    ) -> Result<(Option<Preference>, Vec<Vec<Costs>>)> {
        self.lp.reset().expect("LP Process could not be reset");
        let mut no_constraints = true;
        for c in constraints {
            no_constraints = false;
            self.lp.add_constraint(c)?;
        }

        let mut constraints_by_path: Vec<Vec<Costs>> = vec![Vec::new(); paths.clone().count()];

        let mut alpha;

        let mut repeating_constraints = false;
        loop {
            match self.lp.solve(repeating_constraints)? {
                Some((pref, delta)) => {
                    if delta + ACCURACY < 0.0 {
                        return Ok((None, constraints_by_path));
                    }
                    alpha = pref;
                }
                None => {
                    if no_constraints {
                        alpha = equal_weights(dijkstra.graph.dim.try_into().unwrap());
                        no_constraints = false;
                    } else {
                        return Ok((None, constraints_by_path));
                    }
                }
            }
            repeating_constraints = false;

            let paths = paths.clone();
            let mut sum_dif = 0.0;
            let mut no_constraints = true;
            for (i, path) in paths.enumerate() {
                let res = find_constraint_for_path(path, dijkstra, &alpha);
                sum_dif += res.dif;
                if !float_eq!(res.dif, 0.0) {
                    self.lp.add_constraint(&res.constraint)?;
                    if constraints_by_path[i]
                        .last()
                        .map_or(false, |l| same_array(l, &res.constraint))
                    {
                        repeating_constraints = true;
                    } else {
                        constraints_by_path[i].push(res.constraint);
                        no_constraints = false;
                    }
                }
            }
            if repeating_constraints {
                continue;
            }
            if sum_dif - ACCURACY <= 0.0 {
                return Ok((Some(alpha), constraints_by_path));
            } else if no_constraints {
                return Ok((None, constraints_by_path));
            }
        }
    }

    pub fn constrained_multi_path_preference_lp_only<'p>(
        &mut self,
        constraints: impl Iterator<Item = &'p Costs>,
    ) -> Result<Option<Preference>> {
        self.lp.reset().expect("LP Process could not be reset");

        let mut no_constraints = true;
        for c in constraints {
            self.lp.add_constraint(c)?;
            no_constraints = false;
        }

        // If no constraints are known any, preference is a valid result
        if no_constraints {
            return Ok(Some(equal_weights(self.dim)));
        }
        match self.lp.solve(false)? {
            Some((pref, delta)) => {
                if delta + ACCURACY < 0.0 {
                    Ok(None)
                } else {
                    Ok(Some(pref))
                }
            }
            None => Ok(None),
        }
    }
}

#[derive(Debug)]
struct PathCheckResult {
    dif: f64,
    constraint: Costs,
}

fn find_constraint_for_path(
    path: &Path,
    dijkstra: &mut Dijkstra,
    alpha: &[f64],
) -> PathCheckResult {
    let result = find_path(
        dijkstra,
        &[*path.nodes.first().unwrap(), *path.nodes.last().unwrap()],
        alpha,
    )
    .unwrap();

    let dif = costs_by_alpha(&path.total_dimension_costs, alpha)
        - costs_by_alpha(&result.total_dimension_costs, alpha);

    let cost_dif = result
        .total_dimension_costs
        .iter()
        .zip(path.total_dimension_costs.iter())
        .map(|(r, p)| r - p)
        .collect::<Vec<_>>();

    PathCheckResult {
        dif,
        constraint: cost_dif.into(),
    }
}

pub struct PrefSizeApproximator<'b, 'g, 'd> {
    lp: &'b mut SizeApproxLp,
    dijkstra: &'d mut Dijkstra<'g>,
}

#[derive(Debug, Clone)]
pub struct SizeApproximation {
    pub inner_points: MyVec<Preference>,
    pub point_constraints: Vec<Vec<Vec<f64>>>,
    pub outer_constraints: MyVec<Costs>,
}

#[derive(Debug)]
pub struct ApproxPoint {
    pub point: Preference,
    pub constraints: Vec<Vec<f64>>,
}
impl ApproxPoint {
    pub fn inner_from_size_approximation(sa: &SizeApproximation) -> Vec<Self> {
        let mut approx_points = Vec::new();
        for i in 0..sa.inner_points.len() {
            let mut a_point = ApproxPoint {
                point: sa.inner_points[i].clone(),
                constraints: sa.point_constraints[i].clone(),
            };

            a_point
                .constraints
                .iter_mut()
                .for_each(|c| *c.last_mut().unwrap() *= -1.0);
            approx_points.push(a_point);
        }
        let center = center_point(&sa.inner_points);
        approx_points.sort_by_cached_key(|p| OrderedFloat(angle(&p.point, &center)));
        approx_points.dedup_by(|a, b| {
            same_array(&a.constraints[0], &b.constraints[0])
                && same_array(&a.constraints[1], &b.constraints[1])
                || same_array(&a.constraints[1], &b.constraints[0])
                    && same_array(&a.constraints[0], &b.constraints[1])
        });
        approx_points
    }

    pub fn outer_from_size_approximation(sa: &SizeApproximation) -> Vec<Self> {
        let mut approx_points = Vec::new();

        for (i, j) in sa
            .point_constraints
            .iter()
            .flatten()
            .chain(&[
                vec![0.0, -1.0, -0.0],
                vec![-1.0, 0.0, -0.0],
                vec![-1.0, -1.0, -1.0],
            ])
            .tuple_combinations()
        {
            if same_array(i, j) {
                continue;
            }
            let point = match intersection(i, j) {
                Some(point) => point,
                None => continue,
            };
            assert_eq!(point.len(), 2);
            if 0.0 > point[0] || point[0] > 1.0 || 0.0 > point[1] || point[1] > 1.0 {
                continue;
            }
            if sa
                .point_constraints
                .iter()
                .flatten()
                .all(|c| orientation_test(c, &point) == Orientation::Inside)
            {
                let mut first = i.clone();
                let mut second = j.clone();

                *first.last_mut().unwrap() *= -1.0;
                *second.last_mut().unwrap() *= -1.0;
                approx_points.push(ApproxPoint {
                    point,
                    constraints: vec![first, second],
                });
            }
        }

        let center = center_point(&sa.inner_points);
        approx_points.sort_by_cached_key(|p| OrderedFloat(angle(&p.point, &center)));
        approx_points.dedup_by(|a, b| {
            same_array(&a.constraints[0], &b.constraints[0])
                && same_array(&a.constraints[1], &b.constraints[1])
                || same_array(&a.constraints[1], &b.constraints[0])
                    && same_array(&a.constraints[0], &b.constraints[1])
        });
        approx_points
    }
}

#[metered(registry = ApproxMetrics, registry_expr = APPROX_METRICS )]
impl<'d, 'b, 'g> PrefSizeApproximator<'b, 'g, 'd> {
    pub fn new(lp: &'b mut SizeApproxLp, dijkstra: &'d mut Dijkstra<'g>) -> Self {
        lp.reset().expect("Could not reset lp");
        PrefSizeApproximator { lp, dijkstra }
    }
    #[measure(SimpleTime)]
    pub fn constrained_approx(
        &mut self,
        path: &Path,
        dir_iter: impl Iterator<Item = Preference>,
        constraints: &[Costs],
    ) -> Result<SizeApproximation> {
        self.lp.reset().expect("Could not reset lp");

        let mut no_constraints = true;
        let mut inner_points: MyVec<Preference> = MyVec::new();
        let mut point_constraints = Vec::new();
        let mut outer_constraints = MyVec::new();
        for c in constraints {
            no_constraints = false;
            self.lp.add_constraint(c)?;
            outer_constraints.push(c.clone());
        }

        let dim: usize = self.dijkstra.graph.dim.try_into().unwrap();

        for dir in dir_iter {
            assert_eq!(dir.len(), dim - 1);
            self.lp.set_obj_fun(&dir)?;

            let mut repeating_constraints = false;
            loop {
                let alpha = match self.lp.solve(repeating_constraints)? {
                    Some(alpha) => alpha,
                    None => {
                        if no_constraints {
                            equal_weights(self.dijkstra.graph.dim.try_into().unwrap())
                        } else {
                            panic!("could not find alpha")
                        }
                    }
                };
                repeating_constraints = false;

                let res = find_constraint_for_path(path, &mut self.dijkstra, &alpha);
                if float_eq!(res.dif, 0.0) {
                    inner_points.push(alpha);
                    point_constraints.push(self.lp.non_basic_constraints()?);
                    break;
                }

                if outer_constraints
                    .last()
                    .map_or(false, |l| same_array(l, &res.constraint))
                {
                    repeating_constraints = true;
                    continue;
                }
                self.lp.add_constraint(&res.constraint)?;
                outer_constraints.push(res.constraint);
            }
        }

        Ok(SizeApproximation {
            inner_points,
            point_constraints,
            outer_constraints,
        })
    }

    pub fn approx(
        &mut self,
        path: &Path,
        dir_iter: impl Iterator<Item = Preference>,
    ) -> Result<SizeApproximation> {
        self.constrained_approx(path, dir_iter, &[])
    }
}

pub fn dir_iter(dim: usize, epsilon: usize) -> impl Iterator<Item = Preference> + Clone {
    let angle = 360.0 / epsilon as f64;

    (0..dim).tuple_combinations().flat_map(move |(a, b)| {
        let rot_mat = rotation_matrix(dim, a, b, angle);
        let mut res: MyVec<f64> = vec![0.; dim].into();
        res[a] = 0.5;
        res[b] = 0.5;
        (0..epsilon).map(move |_| {
            res = &rot_mat * &res;
            res.clone()
        })
    })
}

fn rotation_matrix(dim: usize, axis1: usize, axis2: usize, angle: f64) -> SquareMatrix {
    let angle = angle.to_radians();
    let (sin, cos) = angle.sin_cos();
    let mut res = SquareMatrix::new(dim);
    res[axis1][axis1] = cos;
    res[axis2][axis2] = cos;
    res[axis1][axis2] = -sin;
    res[axis2][axis1] = sin;
    for i in 0..dim {
        if i == axis1 || i == axis2 {
            continue;
        }
        res[i][i] = 1.0;
    }

    res
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::dijkstra::find_shortest_path;

    use crate::graph::parse_minimal_graph_file;

    #[test]
    fn test_rotation_matrix_creation() {
        let mat = rotation_matrix(4, 0, 2, 45.0);

        let angle = 45.0_f64.to_radians();
        let cos_45 = angle.cos();
        let sin_45 = angle.sin();

        let expected = [
            [cos_45, 0., -sin_45, 0.],
            [0., 1., 0., 0.],
            [sin_45, 0., cos_45, 0.],
            [0., 0., 0., 1.],
        ];
        for i in 0..4 {
            assert!(same_array(dbg!(&mat[i]), dbg!(&expected[i])));
        }
    }

    #[test]
    fn test_dir_iter() {
        let dim = 3;
        for dir in dir_iter(dim, 2) {
            let point_5s = dir.iter().filter(|v| float_eq!(v.abs(), 0.5)).count();
            let zeros = dir.iter().filter(|v| float_eq!(v.abs(), 0.)).count();
            assert_eq!(point_5s, 2);
            assert_eq!(zeros, dim - 2);
        }
    }

    #[test]
    fn test_four_paths_without_common_pref() {
        use crate::graph::parse_minimal_graph_file;
        use crate::utils::{BitSet, BitSetFns};

        let graph = parse_minimal_graph_file("resources/simple_pref_cover_test_2").unwrap();
        let mut d = Dijkstra::new(&graph);

        let paths = vec![
            find_shortest_path(&mut d, &[0, 1], &[1., 0., 0., 0.]).unwrap(),
            find_shortest_path(&mut d, &[0, 1], &[0., 1., 0., 0.]).unwrap(),
            find_shortest_path(&mut d, &[0, 1], &[0., 0., 1., 0.]).unwrap(),
            find_shortest_path(&mut d, &[0, 1], &[0., 0., 0., 1.]).unwrap(),
        ];

        let mut set_pref = SetPreferences::new(d, &paths).unwrap();

        let mut set = BitSet::new();
        set.add(0);
        set.add(1);
        set.add(2);
        set.add(3);
        let res = set_pref.subset_preference(set.iter());
        assert!(res.is_ok());
        let option_pref = res.unwrap();
        assert!(option_pref.is_none());
    }

    #[test]
    fn test_exact_pref_space() {
        let graph = parse_minimal_graph_file("resources/lp_only_test_graph").unwrap();

        let mut d = Dijkstra::new(&graph);
        let paths = vec![
            find_shortest_path(&mut d, &[0, 1], &[1., 0., 0.]).unwrap(),
            find_shortest_path(&mut d, &[0, 1], &[0., 0., 1.]).unwrap(),
            find_shortest_path(&mut d, &[0, 1], &[0., 1., 0.]).unwrap(),
        ];
        dbg!(&paths);

        let mut set_pref = SetPreferences::new(d, &paths).unwrap();

        let res = set_pref.exact_pref_spaces();

        assert!(same_array(&res[0][0].point, &[0.25, 0.0, 0.75]));
        assert!(same_array(&res[0][1].point, &[1.0, 0.0, 0.0]));
        assert!(same_array(&res[0][2].point, &[0.25, 0.75, 0.0]));
        assert!(same_array(
            &res[0][3].point,
            &[0.07692307692307665, 0.6923076923076923, 0.23076923076923106,]
        ));

        assert!(same_array(&res[1][0].point, &[0.0, 0.0, 1.0]));
        assert!(same_array(&res[1][1].point, &[0.25, 0.0, 0.75]));
        assert!(same_array(
            &res[1][2].point,
            &[0.07692307692307665, 0.6923076923076923, 0.23076923076923106,]
        ));
        assert!(same_array(&res[1][3].point, &[0.0, 0.75, 0.25]));

        assert!(same_array(&res[2][0].point, &[0.0, 0.75, 0.25]));
        assert!(same_array(
            &res[2][1].point,
            &[0.07692307692307665, 0.6923076923076923, 0.23076923076923106,]
        ));
        assert!(same_array(&res[2][2].point, &[0.25, 0.75, 0.0]));
        assert!(same_array(&res[2][3].point, &[0.0, 1.0, 0.0]));
    }

    #[test]
    // This is a test that needs external resources not checked into git and
    // takes a long time to run in debug builds. Therefore, it is ignored.
    #[ignore]
    fn test_larger_subset_has_preference_bug() {
        use crate::utils::io::read_in_paths;

        let graph = parse_minimal_graph_file("../../graphs/bawu.ch").unwrap();
        let d = Dijkstra::new(&graph);

        let paths = read_in_paths("resources/error_paths.txt").unwrap();

        let mut set_pref = SetPreferences::new(d, &paths).unwrap();

        for i in 0..paths.len() {
            for j in i + 1..paths.len() {
                if i == 1 && j == 3 {
                    println!("this line is only for debugging purposes");
                }
                let res = set_pref.subset_preference([i, j].iter().copied()).unwrap();

                if res.is_none() {
                    assert!(set_pref
                        .subset_preference([0, i, j].iter().copied())
                        .unwrap()
                        .is_none())
                }
            }
        }
    }

    #[test]
    fn test_lp_only_solves_lp_with_right_constraints() {
        let graph = parse_minimal_graph_file("resources/lp_only_test_graph").unwrap();

        let mut d = Dijkstra::new(&graph);
        let paths = vec![
            find_shortest_path(&mut d, &[0, 1], &[1., 0., 0.]).unwrap(),
            find_shortest_path(&mut d, &[0, 1], &[0., 0., 1.]).unwrap(),
            find_shortest_path(&mut d, &[0, 1], &[0., 1., 0.]).unwrap(),
        ];

        dbg!(&paths[0].total_dimension_costs);
        dbg!(&paths[1].total_dimension_costs);
        dbg!(&paths[2].total_dimension_costs);

        let mut set_pref = SetPreferences::new(d, &paths).unwrap();

        set_pref.subset_preference([0].iter().copied()).unwrap();
        set_pref.subset_preference([1].iter().copied()).unwrap();
        set_pref.subset_preference([2].iter().copied()).unwrap();

        let res01_lp = set_pref
            .subset_preference_lp_only([0, 1].iter().copied())
            .unwrap()
            .unwrap();

        let res01_normal = set_pref
            .subset_preference([0, 1].iter().copied())
            .unwrap()
            .unwrap();

        assert!(!same_array(&res01_normal, &res01_lp));
    }
}
