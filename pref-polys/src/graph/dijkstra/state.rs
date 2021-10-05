use std::cmp::Ordering;

use ordered_float::OrderedFloat;

use crate::graph::edge::EdgeDirection;

#[derive(Debug, PartialEq, Clone)]
pub struct State {
    pub node_id: u32,
    pub total_cost: f64,
    pub direction: EdgeDirection,
}

impl State {
    pub fn new(node_id: u32, direction: EdgeDirection) -> Self {
        State {
            node_id,
            total_cost: 0.0,
            direction,
        }
    }
}

impl std::cmp::Eq for State {}

impl std::cmp::Ord for State {
    // switch comparison, because we want a min-heap
    fn cmp(&self, other: &Self) -> Ordering {
        OrderedFloat(other.total_cost).cmp(&OrderedFloat(self.total_cost))
    }
}

impl std::cmp::PartialOrd for State {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
