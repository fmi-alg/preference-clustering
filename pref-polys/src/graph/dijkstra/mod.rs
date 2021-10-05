use std::collections::binary_heap::BinaryHeap;

use state::State;

use crate::graph::{path::costs_by_alpha, Graph};
use crate::utils::metrics::YesNoTime;
use crate::utils::MyVec;

mod ndijkstra;
mod state;

pub use ndijkstra::NDijkstra;

use lazy_static::lazy_static;
use metered::metered;

use super::{
    edge::EdgeDirection,
    path::{add_edge_costs, Path},
};

pub struct HalfPath {
    pub edges: MyVec<MyVec<u32>>,
    pub dimension_costs: MyVec<MyVec<f64>>,
    pub total_dimension_costs: MyVec<f64>,
    pub costs_by_alpha: MyVec<f64>,
}

#[derive(Clone)]
pub struct DijkstraResult {
    pub edges: MyVec<u32>,
    pub costs: MyVec<f64>,
    pub total_cost: f64,
}

#[derive(Clone)]
pub struct Dijkstra<'a> {
    pub graph: &'a Graph,
    candidates: BinaryHeap<State>,
    touched_nodes: MyVec<u32>,
    found_best_b: bool,
    found_best_f: bool,

    // Best dist to/from node
    pub cost_f: MyVec<f64>,
    pub cost_b: MyVec<f64>,

    // Best edge to/from node
    pub previous_f: MyVec<Option<u32>>,
    pub previous_b: MyVec<Option<u32>>,

    // (node_id, cost array, total_cost)
    best_node: Option<(u32, f64)>,
}

#[metered(registry = DijkstraMetrics, registry_expr = DIJKSTRA_METRICS )]
impl<'a> Dijkstra<'a> {
    pub fn new(graph: &Graph) -> Dijkstra {
        let num_of_nodes = graph.nodes.len();
        Dijkstra {
            graph,
            candidates: BinaryHeap::new(),
            touched_nodes: MyVec(Vec::new()),
            found_best_b: false,
            found_best_f: false,
            cost_f: MyVec(vec![std::f64::MAX; num_of_nodes]),
            cost_b: MyVec(vec![std::f64::MAX; num_of_nodes]),
            previous_f: MyVec(vec![None; num_of_nodes]),
            previous_b: MyVec(vec![None; num_of_nodes]),
            best_node: None,
        }
    }

    fn prepare(&mut self, source: u32, target: u32) {
        // Candidates
        self.candidates = BinaryHeap::new();
        self.candidates.push(State::new(source, EdgeDirection::Out));
        self.candidates.push(State::new(target, EdgeDirection::In));

        // Touched nodes
        for node_id in &self.touched_nodes.0 {
            self.cost_f[*node_id] = std::f64::MAX;
            self.cost_b[*node_id] = std::f64::MAX;
            self.previous_f[*node_id] = None;
            self.previous_b[*node_id] = None;
        }
        self.touched_nodes.clear();

        self.found_best_b = false;
        self.found_best_f = false;

        // Node states
        self.cost_f[source] = 0.0;
        self.cost_b[target] = 0.0;
        self.touched_nodes.push(source);
        self.touched_nodes.push(target);

        // Best node
        self.best_node = None;
    }

    #[measure(YesNoTime)]
    pub fn run(&mut self, source: u32, target: u32, alpha: &[f64]) -> Option<DijkstraResult> {
        self.prepare(source, target);

        // let now = Instant::now();
        // let mut n_popped: usize = 0;
        while let Some(candidate) = self.candidates.pop() {
            // n_popped += 1;
            if self.found_best_f && self.found_best_b {
                break;
            }
            self.process_state(candidate, alpha);
        }

        match self.best_node {
            None => None,
            Some((node_id, total_cost)) => {
                /*
                    println!(
                    "Found path with cost {:?} in {:?}ms with {:?} nodes popped",
                    total_cost,
                    now.elapsed().as_millis(),
                    n_popped
                );
                     */
                // println!(
                //     "Found path with dim_costs {:?} and cost {:?}",
                //     costs, total_cost
                // );
                let (edges, costs) = self.make_edge_path(node_id);
                Some(DijkstraResult {
                    edges,
                    costs,
                    total_cost,
                })
            }
        }
    }

    fn process_state(&mut self, candidate: State, alpha: &[f64]) {
        let State {
            node_id,
            total_cost,
            direction,
        } = candidate;

        let my_costs;
        let other_costs;
        let found_best;
        let previous;
        if direction == EdgeDirection::Out {
            my_costs = &mut self.cost_f;
            other_costs = &self.cost_b;
            found_best = &mut self.found_best_f;
            previous = &mut self.previous_f;
        } else {
            my_costs = &mut self.cost_b;
            other_costs = &self.cost_f;
            found_best = &mut self.found_best_b;
            previous = &mut self.previous_b;
        };

        if total_cost > my_costs[node_id] {
            return;
        };
        let best_node_cost = self.best_node.unwrap_or((0, std::f64::MAX)).1;

        if total_cost > best_node_cost {
            *found_best = true;
            return;
        }
        if other_costs[node_id] != std::f64::MAX {
            let merged_cost = total_cost + other_costs[node_id];
            if merged_cost < best_node_cost {
                // let merged_cost_vector = add_edge_costs(costs, other_costs[node_id].0);
                self.best_node = Some((node_id, merged_cost));
            }
        }

        let edges = self.graph.edges_of(node_id, direction);
        for half_edge in edges {
            if self.graph.nodes[node_id].ch_level > self.graph.nodes[half_edge.target_id].ch_level {
                break;
            }

            let next_node = half_edge.target_id;
            let next_total_cost = total_cost + costs_by_alpha(half_edge.edge_costs, alpha);

            if next_total_cost < my_costs[next_node] {
                my_costs[next_node] = next_total_cost;
                previous[next_node] = Some(half_edge.edge_id);
                self.touched_nodes.push(next_node);
                self.candidates.push(State {
                    node_id: next_node,
                    total_cost: next_total_cost,
                    direction,
                });
            }
        }
    }

    fn make_edge_path(&self, connector: u32) -> (MyVec<u32>, MyVec<f64>) {
        let mut edges = MyVec(Vec::new());
        let mut previous_edge = self.previous_f[connector];
        let mut successive_edge = self.previous_b[connector];

        let mut costs: MyVec<_> = vec![0.0; self.graph.dim as usize].into();

        // backwards
        while let Some(edge_id) = previous_edge {
            edges.push(edge_id);
            add_edge_costs(&mut costs, &self.graph.edges[edge_id].edge_costs);
            previous_edge = self.previous_f[self.graph.edges[edge_id].source_id];
        }
        edges.reverse();

        // forwards
        while let Some(edge_id) = successive_edge {
            edges.push(edge_id);
            add_edge_costs(&mut costs, &self.graph.edges[edge_id].edge_costs);
            successive_edge = self.previous_b[self.graph.edges[edge_id].target_id];
        }
        (edges, costs)
    }
}

pub fn find_path(dijkstra: &mut Dijkstra, include: &[u32], alpha: &[f64]) -> Option<HalfPath> {
    // println!("=== Running Dijkstra search ===");
    let mut edges = MyVec::new();
    let mut dimension_costs = MyVec::new();
    let mut total_dimension_costs: MyVec<f64> = vec![0.0; dijkstra.graph.dim as usize].into();
    let mut costs_by_alpha = MyVec::new();

    for win in include.windows(2) {
        if let Some(result) = dijkstra.run(win[0], win[1], alpha) {
            edges.push(result.edges);
            result
                .costs
                .iter()
                .enumerate()
                .for_each(|(index, val)| total_dimension_costs[index] += *val);
            dimension_costs.push(result.costs);
            costs_by_alpha.push(result.total_cost);
        } else {
            return None;
        }
    }
    Some(HalfPath {
        edges,
        dimension_costs,
        total_dimension_costs,
        costs_by_alpha,
    })
}

pub fn find_shortest_path(dijkstra: &mut Dijkstra, include: &[u32], alpha: &[f64]) -> Option<Path> {
    if let Some(result) = find_path(dijkstra, include, alpha) {
        let graph = dijkstra.graph;
        let unpacked_edges = result.edges.iter().map(|subpath_edges| {
            subpath_edges
                .iter()
                .flat_map(|edge| graph.unpack_edge(*edge))
                .collect::<Vec<_>>()
        });

        let edges: Vec<u32> = unpacked_edges.flatten().collect();
        let mut nodes: Vec<u32> = edges
            .iter()
            .map(|edge| graph.edges[*edge].source_id)
            .collect();
        nodes.push(*include.last().unwrap());
        return Some(Path {
            nodes: MyVec(nodes),
            edges: MyVec(edges),
            total_dimension_costs: result.total_dimension_costs,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use std::convert::TryInto;

    use super::*;
    use crate::graph::{parse_graph_file, path::randomized_preference, Graph};

    fn get_conc_graph() -> Graph {
        parse_graph_file("./resources/concTestGraph").unwrap()
    }

    #[test]
    fn normal_case() {
        use crate::float_eq;

        let conc_graph = get_conc_graph();
        let mut dijkstra = NDijkstra::new(&conc_graph);
        let mut dijkstra_conc = Dijkstra::new(&conc_graph);

        let mut rng = rand::thread_rng();
        for s in 0..(conc_graph.nodes.len() as u32) {
            for t in 0..(conc_graph.nodes.len() as u32) {
                let alpha = randomized_preference(conc_graph.dim.try_into().unwrap(), &mut rng);
                let ch_path = dijkstra_conc.run(s, t, &alpha);
                let alpha = alpha.0.try_into().unwrap();
                let n_costs = dijkstra.run(s, t, &alpha);

                assert!(float_eq!(
                    ch_path.as_ref().map(|r| r.total_cost).unwrap_or(-1.),
                    n_costs.unwrap_or(-1.)
                ));

                let n_path = dijkstra.path(t);

                let unpacked_path = ch_path.map(|r| {
                    r.edges
                        .0
                        .into_iter()
                        .flat_map(|e| conc_graph.unpack_edge(e))
                        .collect()
                });
                let n_path_edges = n_path.map(|r| r.edges.0);

                // We dont assert equality here because there might exists equal cost paths
                if unpacked_path != n_path_edges {
                    dbg!(unpacked_path, n_path_edges);
                }
            }
        }
    }
}

lazy_static! {
    static ref DIJKSTRA_METRICS: DijkstraMetrics = Default::default();
}

pub struct TimeReports;

impl TimeReports {
    pub fn dijkstra() {
        println!("Dijkstra report:",);
        println!("{}", DIJKSTRA_METRICS.run.yes_no_time);
        println!("----------");
    }

    pub fn clear_dijkstra_time() {
        use metered::clear::Clear;
        DIJKSTRA_METRICS.run.yes_no_time.clear();
    }
}
