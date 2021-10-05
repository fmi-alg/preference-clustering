use std::convert::TryInto;

use rand::{
    distributions::Uniform,
    prelude::{Distribution, SliceRandom, ThreadRng},
};
use serde::{Deserialize, Serialize};

use crate::graph::Graph;
use crate::utils::MyVec;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Path {
    pub nodes: MyVec<u32>,
    pub edges: MyVec<u32>,
    pub total_dimension_costs: MyVec<f64>,
}

pub fn add_edge_costs(a: &mut [f64], b: &[f64]) {
    a.iter_mut().zip(b).for_each(|(a, b)| *a += b)
}

pub fn costs_by_alpha(costs: &[f64], alpha: &[f64]) -> f64 {
    assert_eq!(costs.len(), alpha.len());
    // The unsafe version of this hot loop safes a lot of runtime ...
    // The idiomatic version with iterators was about 10% slower :(
    let mut res = 0.0;
    for i in 0..costs.len() {
        // SAFETY: By above assert costs and alpha have the same length and get
        // accessed only on valid indices.
        unsafe {
            res += costs.get_unchecked(i) * alpha.get_unchecked(i);
        }
    }
    res
}

pub fn randomized_preference(dim: usize, rng: &mut ThreadRng) -> MyVec<f64> {
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

impl Path {
    pub fn get_subpath_costs(&self, graph: &Graph, start: u32, end: u32) -> MyVec<f64> {
        let edges = &self.edges[start..end];
        let mut res = vec![0.0; graph.dim.try_into().unwrap()];
        edges.iter().for_each(|e| {
            res.iter_mut()
                .zip(&graph.edges[*e].edge_costs)
                .for_each(|(r, e)| *r += e)
        });

        res.into()
    }
    pub fn get_subpath(&self, graph: &Graph, start: u32, end: u32) -> Path {
        let nodes = MyVec(self.nodes[start..end].iter().copied().collect());
        let mut stop = end;
        if stop > 0 {
            stop -= 1;
        }
        let edges = MyVec(self.edges[start..stop].iter().copied().collect());
        let total_dimension_costs = self.get_subpath_costs(graph, start, stop);
        Path {
            nodes,
            edges,
            total_dimension_costs,
        }
    }
}

#[test]
fn test_add_edge_costs() {
    let mut a = [1.5, 2.0, 0.7, 1.3];
    let b = [1.3, 0.1, 0.3, 0.3];
    add_edge_costs(&mut a, &b);
    assert!(crate::utils::same_array(&[2.8, 2.1, 1.0, 1.6], &a));
}
