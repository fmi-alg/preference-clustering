use std::{fs::File, io::BufReader, path::PathBuf, time::Instant};

use graph::path::Path;
use structopt::StructOpt;

use pref_polys::graph;
use pref_polys::graph::dijkstra::Dijkstra;
use pref_polys::utils::randomized_preference;

use anyhow::Result;
use rand::distributions::{Distribution, Uniform};
use rand::rngs::StdRng;
use rand::{thread_rng, RngCore, SeedableRng};

use std::convert::TryInto;

use std::io::Write;

static PRECISION: f64 = 0.00000001;

#[derive(StructOpt)]
struct Opts {
    /// Path to the graph file
    graph: PathBuf,
    #[structopt(short = "s", long)]
    /// seed for randomly generated trajectories
    seed: Option<u64>,
    /// Path to output file
    #[structopt(short = "o", long)]
    output: Option<String>,
    #[structopt(short = "d", long)]
    debug_output: Option<String>,
    /// Used mode:
    ///   - 0: generate 'p' trajectories with 'n' different preferences,
    ///        output preference spaces (default)
    ///   - 1: generate 1 trajectory, output area representation
    ///   - 2: generate 'p' trajectories, output most complex area representation
    ///   - 3: read paths file, output preference spaces
    #[structopt(short = "m", long, verbatim_doc_comment)]
    modus: Option<u32>,
    /// Amount of preferences to use when generating trajectories
    #[structopt(short = "n", long)]
    num_prefs: Option<u32>,
    /// Amount of trajectories to generate
    #[structopt(short = "p", long)]
    num_paths: Option<u32>,
    /// Path to paths files
    #[structopt(short = "f", long)]
    path_file: Option<PathBuf>,
}

pub struct Corner {
    coords: Vec<f64>,
    is_checked: bool,
    neighbor_indices: Vec<usize>,
    constraint_indices: Vec<usize>,
}
impl Corner {
    fn new() -> Self {
        Corner {
            coords: Vec::new(),
            is_checked: false,
            neighbor_indices: Vec::new(),
            constraint_indices: Vec::new(),
        }
    }
}

pub struct AreaCalculator {
    source: u32,
    target: u32,
    costs: Vec<u32>,
    corners: Vec<Corner>,
    hull_indices: Vec<usize>,
    ch_counter: usize,
    constraints: Vec<Vec<i32>>,
}

impl AreaCalculator {
    fn new(source: u32, target: u32, costs: Vec<u32>) -> Self {
        let mut corners = Vec::new();
        let mut hull_indices = Vec::new();
        let mut constraints = Vec::new();
        for i in 0..3 {
            let mut constraint = vec![0; 3];
            constraint[i] = -1;
            constraints.push(constraint);
            let mut coords = vec![0.; 3];
            let neighbor_indices = vec![(i + 1) % 3, (i + 2) % 3];
            let constraint_indices = vec![(i + 2) % 3, (i + 1) % 3];
            coords[i] = 1.;
            corners.push(Corner {
                coords,
                is_checked: false,
                neighbor_indices,
                constraint_indices,
            });
            hull_indices.push(i);
        }
        AreaCalculator {
            source,
            target,
            costs,
            corners,
            hull_indices,
            ch_counter: 0,
            constraints,
        }
    }

    fn calculate_area(&mut self, dijkstra: &mut Dijkstra, debug: bool) {
        let mut hull_index: usize = 0;
        let mut counter = 0;
        while hull_index < self.hull_indices.len() {
            let corner_index = self.hull_indices[hull_index];
            let cost = get_cost_vector_for_pref(
                dijkstra,
                &self.corners[corner_index].coords,
                self.source,
                self.target,
            );
            self.ch_counter += 1;
            self.corners[corner_index].is_checked = true;
            let mut cost_diff = Vec::new();
            for i in 0..cost.len() {
                cost_diff.push(self.costs[i] as i32 - cost[i] as i32);
            }
            let mut aggregated_diff = 0.;
            for i in 0..cost_diff.len() {
                aggregated_diff += cost_diff[i] as f64 * self.corners[corner_index].coords[i];
            }
            if aggregated_diff > PRECISION {
                let mut dot_products: Vec<f64> = vec![0.; self.corners.len()];
                let mut new_hull_indices = Vec::new();
                for hi in &self.hull_indices {
                    let mut dot_p = 0.;
                    for i in 0..cost_diff.len() {
                        dot_p += cost_diff[i] as f64 * self.corners[*hi].coords[i];
                    }
                    dot_products[*hi] = dot_p;
                    if dot_p <= PRECISION {
                        new_hull_indices.push(*hi);
                    }
                }
                self.constraints.push(cost_diff);
                let mut index_right = self.corners.len();
                let mut index_left = self.corners.len() + 1;
                {
                    let mut first_in = corner_index;
                    while dot_products[first_in] > PRECISION {
                        first_in = self.corners[first_in].neighbor_indices[0];
                    }
                    if dot_products[first_in] >= 0. || -dot_products[first_in] <= PRECISION {
                        index_right = first_in;
                        index_left = self.corners.len();
                    } else {
                        let mut new_corner = Corner::new();
                        new_corner.neighbor_indices.push(first_in);
                        new_corner.neighbor_indices.push(index_left);
                        new_corner
                            .constraint_indices
                            .push(self.corners[first_in].constraint_indices[1]);
                        new_corner
                            .constraint_indices
                            .push(self.constraints.len() - 1);
                        let out_index = self.corners[first_in].neighbor_indices[1];
                        self.corners[first_in].neighbor_indices[1] = index_right;
                        let dot1 = dot_products[first_in];
                        let dot2 = dot_products[out_index];
                        {
                            let p = dot2 / (dot2 - dot1);
                            for i in 0..self.corners[first_in].coords.len() {
                                let x = p * self.corners[first_in].coords[i]
                                    + (1. - p) * self.corners[out_index].coords[i];
                                new_corner.coords.push(x);
                            }
                        }
                        self.corners.push(new_corner);
                        new_hull_indices.push(index_right);
                    }
                }
                {
                    let mut first_in = corner_index;
                    while dot_products[first_in] > PRECISION {
                        first_in = self.corners[first_in].neighbor_indices[1];
                    }
                    if dot_products[first_in] >= 0. || -dot_products[first_in] <= PRECISION {
                        index_left = first_in;
                    } else {
                        let mut new_corner = Corner::new();
                        new_corner.neighbor_indices.push(index_right);
                        new_corner.neighbor_indices.push(first_in);
                        new_corner
                            .constraint_indices
                            .push(self.constraints.len() - 1);
                        new_corner
                            .constraint_indices
                            .push(self.corners[first_in].constraint_indices[0]);
                        let out_index = self.corners[first_in].neighbor_indices[0];
                        self.corners[first_in].neighbor_indices[0] = index_left;
                        let dot1 = dot_products[first_in];
                        let dot2 = dot_products[out_index];
                        for i in 0..self.corners[first_in].coords.len() {
                            let p = dot2 / (dot2 - dot1);
                            let x = p * self.corners[first_in].coords[i]
                                + (1. - p) * self.corners[out_index].coords[i];
                            new_corner.coords.push(x);
                        }
                        self.corners.push(new_corner);
                        new_hull_indices.push(index_left);
                    }
                }
                self.corners[index_left].neighbor_indices[0] = index_right;
                self.corners[index_left].constraint_indices[0] = self.constraints.len() - 1;
                self.corners[index_right].neighbor_indices[1] = index_left;
                self.corners[index_right].constraint_indices[1] = self.constraints.len() - 1;
                self.hull_indices = new_hull_indices;
            }
            if debug {
                println!("num corners: {}", self.corners.len());
                self.print_hull();
            }
            hull_index = 0;
            while hull_index < self.hull_indices.len()
                && self.corners[self.hull_indices[hull_index]].is_checked
            {
                hull_index += 1;
            }
            counter += 1;
            if debug {
                self.print_area_to_file(String::from(format!("debug-{}.txt", counter)))
                    .unwrap();
            }
        }
    }

    fn print_hull(&self) {
        print!("hull:");
        for hi in &self.hull_indices {
            print!(" {}", *hi);
        }
        println!();
        println!("neighbors:");
        for hi in &self.hull_indices {
            println!(
                "{} {}",
                self.corners[*hi].neighbor_indices[0], self.corners[*hi].neighbor_indices[1]
            );
        }
    }

    fn get_intersections(&self) -> Vec<Vec<Vec<i32>>> {
        let mut intersections = Vec::new();
        if self.hull_indices.len() == 0 {
            return intersections;
        }
        let first_index = self.hull_indices[0];
        let mut index = first_index;
        let mut finished = false;
        while !finished {
            let mut intersection: Vec<Vec<i32>> = Vec::new();
            intersection.push(self.constraints[self.corners[index].constraint_indices[0]].clone());
            intersection.push(self.constraints[self.corners[index].constraint_indices[1]].clone());
            intersections.push(intersection);
            index = self.corners[index].neighbor_indices[0];
            if index == first_index {
                finished = true;
            }
        }
        intersections
    }

    fn get_area_as_string(&self) -> String {
        let mut index_map = vec![0 as usize; self.corners.len()];
        for i in 0..self.hull_indices.len() {
            index_map[self.hull_indices[i]] = i;
        }
        let mut content: String = String::from("");
        for hi in &self.hull_indices {
            content = format!(
                "{}{} {} 2 {} {}\n",
                content,
                self.corners[*hi].coords[0],
                self.corners[*hi].coords[1],
                index_map[self.corners[*hi].neighbor_indices[0]],
                index_map[self.corners[*hi].neighbor_indices[1]]
            );
        }
        content
    }

    fn get_area_as_string_with_offset(&self, offset: usize) -> String {
        let mut index_map = vec![0 as usize; self.corners.len()];
        for i in 0..self.hull_indices.len() {
            index_map[self.hull_indices[i]] = i;
        }
        let mut content: String = String::from("");
        for hi in &self.hull_indices {
            content = format!(
                "{}{} {} 2 {} {}\n",
                content,
                self.corners[*hi].coords[0],
                self.corners[*hi].coords[1],
                index_map[self.corners[*hi].neighbor_indices[0]] + offset,
                index_map[self.corners[*hi].neighbor_indices[1]] + offset
            );
        }
        content
    }

    fn print_area_to_file(&self, file_name: String) -> std::io::Result<()> {
        let content = self.get_area_as_string();
        let mut file = File::create(file_name)?;
        file.write_all(content.as_bytes())?;
        Ok(())
    }

    /*fn print_hull(&self) {
        print!("hull indices:");
        for hl in &self.hull_indices {
            print!(" {}", *hl);
        }
        println!();
    }*/

    fn is_inside(&self, coords: &Vec<f64>) -> bool {
        for c in &self.constraints {
            let mut dot_p = 0.;
            for i in 0..coords.len() {
                dot_p += c[i] as f64 * coords[i];
            }
            if dot_p > PRECISION {
                return false;
            }
        }
        true
    }

    fn debug_constraints(&self) {
        for corner in &self.corners {
            let mut dot_p1 = 0.;
            let mut dot_p2 = 0.;
            let index1 = corner.constraint_indices[0];
            let index2 = corner.constraint_indices[1];
            for i in 0..self.constraints[index1].len() {
                dot_p1 += self.constraints[index1][i] as f64 * corner.coords[i];
                dot_p2 += self.constraints[index2][i] as f64 * corner.coords[i];
            }
            println!("This should be 0: {} {}", dot_p1, dot_p2);
        }
    }

    fn debug_area(
        &mut self,
        dijkstra: &mut Dijkstra,
        output_file: String,
        seed: u64,
        num_runs: u32,
    ) -> std::io::Result<()> {
        println!("Start debugging area...");
        let mut rng = StdRng::seed_from_u64(seed);
        let mut debug_content: String = String::from("");
        for _ in 0..num_runs {
            let my_pref = randomized_preference(&mut rng, dijkstra.graph.dim.try_into().unwrap());
            let mut pref = Vec::new();
            for i in 0..my_pref.len() {
                pref.push(my_pref[i]);
            }
            let is_inside = self.is_inside(&pref);
            let mut new_costs: Vec<u32> = Vec::new();
            if let Some(result) = dijkstra.run(self.source, self.target, &pref) {
                for i in 0..result.costs.len() {
                    new_costs.push(result.costs[i] as u32);
                }
            }
            let mut dot_p = 0.;
            for i in 0..self.costs.len() {
                dot_p += pref[i] * (self.costs[i] as i32 - new_costs[i] as i32) as f64;
            }
            let is_really_inside = dot_p <= 0.;
            if is_really_inside != is_inside {
                println!("ERROR: {} {} {}", is_inside, is_really_inside, dot_p);
                /*println!("costs: {} {} {}", costs[0], costs[1], costs[2]);
                println!(
                    "new costs: {} {} {}",
                    new_costs[0], new_costs[1], new_costs[2]
                );
                println!("pref: {} {} {}", pref[0], pref[1], pref[2]);*/
            }
            if is_inside {
                debug_content = format!("{}{} {} y\n", debug_content, my_pref[0], my_pref[1],);
            } else {
                debug_content = format!("{}{} {} n\n", debug_content, my_pref[0], my_pref[1],);
            }
        }
        println!("Finished.");
        let mut file = File::create(output_file)?;
        file.write_all(debug_content.as_bytes())?;
        Ok(())
    }
}

fn get_all_optimal_areas_as_string(
    dijk: &mut graph::dijkstra::Dijkstra,
    source: u32,
    target: u32,
) -> String {
    let mut content: String = String::from("");
    let mut optimal_paths: Vec<Vec<u32>> = Vec::new();
    let default_preference = vec![1., 0., 0.];
    optimal_paths.push(get_cost_vector_for_pref(
        dijk,
        &default_preference,
        source,
        target,
    ));
    let mut index: usize = 0;
    let mut num_corners = 0;
    while index < optimal_paths.len() {
        let mut calculator = AreaCalculator::new(source, target, optimal_paths[index].clone());
        calculator.calculate_area(dijk, false);
        for i in 3..calculator.constraints.len() {
            let constraint = &calculator.constraints[i];
            let mut new_costs = vec![0; 3];
            for j in 0..3 {
                new_costs[j] = (optimal_paths[index][j] as i32 - constraint[j]) as u32;
            }
            let mut is_new = true;
            for j in 0..optimal_paths.len() {
                if optimal_paths[j][0] == new_costs[0]
                    && optimal_paths[j][1] == new_costs[1]
                    && optimal_paths[j][2] == new_costs[2]
                {
                    is_new = false;
                    break;
                }
            }
            if is_new {
                println!("add new path: {:?}", new_costs);
                optimal_paths.push(new_costs);
            }
        }
        content = format!(
            "{}{}",
            content,
            calculator.get_area_as_string_with_offset(num_corners)
        );
        num_corners += calculator.hull_indices.len();
        index += 1;
        println!("found paths: {}\nindex: {}\n", optimal_paths.len(), index);
    }
    content
}

fn get_all_optimal_costs_as_string(
    dijk: &mut graph::dijkstra::Dijkstra,
    source: u32,
    target: u32,
) -> String {
    let mut content: String = String::from("");
    let mut optimal_paths: Vec<Vec<u32>> = Vec::new();
    let default_preference = vec![1., 0., 0.];
    optimal_paths.push(get_cost_vector_for_pref(
        dijk,
        &default_preference,
        source,
        target,
    ));
    let mut index: usize = 0;
    while index < optimal_paths.len() {
        let mut calculator = AreaCalculator::new(source, target, optimal_paths[index].clone());
        calculator.calculate_area(dijk, false);
        for i in 3..calculator.constraints.len() {
            let constraint = &calculator.constraints[i];
            let mut new_costs = vec![0; 3];
            for j in 0..3 {
                new_costs[j] = (optimal_paths[index][j] as i32 - constraint[j]) as u32;
            }
            let mut is_new = true;
            for j in 0..optimal_paths.len() {
                if optimal_paths[j][0] == new_costs[0]
                    && optimal_paths[j][1] == new_costs[1]
                    && optimal_paths[j][2] == new_costs[2]
                {
                    is_new = false;
                    break;
                }
            }
            if is_new {
                println!("add new path: {:?}", new_costs);
                optimal_paths.push(new_costs);
            }
        }
        index += 1;
        println!("found paths: {}\nindex: {}\n", optimal_paths.len(), index);
    }
    for path in optimal_paths {
        content = format!("{}{} {} {}\n", content, path[0], path[1], path[2]);
    }
    content
}

fn get_cost_vector_for_pref(
    dijk: &mut graph::dijkstra::Dijkstra,
    preference: &[f64],
    source: u32,
    target: u32,
) -> Vec<u32> {
    let res = dijk.run(source, target, preference).unwrap();
    let mut costs: Vec<u32> = Vec::new();
    for i in 0..res.costs.len() {
        costs.push(res.costs[i] as u32);
    }
    costs
}

fn print_intersections_to_file(
    file_name: String,
    intersections: Vec<Vec<Vec<Vec<i32>>>>,
) -> std::io::Result<()> {
    let mut content: String;
    content = format!("{}\n", intersections.len(),);
    for poly in intersections {
        content = format!("{}{}", content, poly.len(),);
        for intersection in poly {
            for constraint in intersection {
                content = format!(
                    "{} {} {} {}",
                    content,
                    constraint[0] - constraint[2],
                    constraint[1] - constraint[2],
                    constraint[2],
                );
            }
        }
        content = format!("{}\n", content,);
    }
    let mut file = File::create(file_name)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

fn main() -> Result<()> {
    let opts = Opts::from_args();
    let graph = graph::parse_minimal_graph_file(&opts.graph)?;
    let mut t_rng = thread_rng();
    let seed = opts.seed.unwrap_or_else(|| t_rng.next_u64());
    println!("using seed {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);
    let modus = opts.modus.unwrap_or_else(|| 0);
    let output = opts.output.unwrap_or_else(|| String::from("output.txt"));

    let mut dijk = graph::dijkstra::Dijkstra::new(&graph);
    if modus == 0 {
        let num_paths = opts.num_paths.unwrap_or_else(|| 10);
        let num_prefs = opts.num_prefs.unwrap_or_else(|| 5);
        let mut prefs = Vec::new();
        for _ in 0..num_prefs {
            let pref = randomized_preference(&mut rng, graph.dim.try_into().unwrap());
            prefs.push(pref);
        }
        let mut sum_ch = 0;
        let mut intersections = Vec::new();
        for i in 0..num_paths {
            let nodes_dist = Uniform::from(0..dijk.graph.nodes.len() as u32);
            let pref_dist = Uniform::from(0..prefs.len() as u32);
            let mut s = 0;
            let mut t = 0;
            let mut costs = Vec::new();
            while costs.len() == 0 {
                s = nodes_dist.sample(&mut rng);
                t = nodes_dist.sample(&mut rng);
                let pref_index = pref_dist.sample(&mut rng);
                let pref = prefs[pref_index as usize].clone();
                if let Some(result) = dijk.run(s, t, &pref) {
                    for i in 0..result.costs.len() {
                        costs.push(result.costs[i] as u32);
                    }
                }
            }
            let mut area_calculator = AreaCalculator::new(s, t, costs.clone());
            area_calculator.calculate_area(&mut dijk, false);
            intersections.push(area_calculator.get_intersections());
            sum_ch += area_calculator.ch_counter;
            println!("{}: {}", i, area_calculator.ch_counter);
        }
        print_intersections_to_file(output, intersections)?;
        println!(
            "Finished. Average CH calls: {}",
            sum_ch / num_paths as usize
        );
    } else if modus == 1 {
        let nodes_dist = Uniform::from(0..dijk.graph.nodes.len() as u32);
        let mut s = 0;
        let mut t = 0;
        let mut costs = Vec::new();
        while costs.len() == 0 {
            s = nodes_dist.sample(&mut rng);
            t = nodes_dist.sample(&mut rng);
            let pref = randomized_preference(&mut rng, dijk.graph.dim.try_into().unwrap());
            if let Some(result) = dijk.run(s, t, &pref) {
                for i in 0..result.costs.len() {
                    costs.push(result.costs[i] as u32);
                }
            }
            println!("pref: {:?}", pref);
        }
        let mut area_calculator = AreaCalculator::new(s, t, costs);
        println!("Compute area...");
        area_calculator.calculate_area(&mut dijk, false);
        println!("Finished. CH calls: {}", area_calculator.ch_counter);
        area_calculator.debug_constraints();
        let debug_file = opts
            .debug_output
            .unwrap_or_else(|| String::from("debug.txt"));
        area_calculator.debug_area(&mut dijk, debug_file, seed, 1000)?;
        area_calculator.print_area_to_file(output)?;
    } else if modus == 2 {
        let num_paths = opts.num_paths.unwrap_or_else(|| 100);
        let mut s = 0;
        let mut t = 0;
        let mut most_complicated_area = AreaCalculator::new(s, t, Vec::new());
        for i in 0..num_paths {
            let nodes_dist = Uniform::from(0..graph.nodes.len() as u32);
            let mut costs = Vec::new();
            while costs.len() == 0 {
                s = nodes_dist.sample(&mut rng);
                t = nodes_dist.sample(&mut rng);
                let pref = randomized_preference(&mut rng, graph.dim.try_into().unwrap());
                if let Some(result) = dijk.run(s, t, &pref) {
                    for i in 0..result.costs.len() {
                        costs.push(result.costs[i] as u32);
                    }
                }
                println!("pref: {:?}", pref);
            }
            let mut area_calculator = AreaCalculator::new(s, t, costs.clone());
            println!("Compute area...");
            area_calculator.calculate_area(&mut dijk, false);
            println!("Finished. CH calls: {}", area_calculator.ch_counter);
            if i == 0
                || most_complicated_area.hull_indices.len() < area_calculator.hull_indices.len()
            {
                most_complicated_area = area_calculator;
            }
        }
        most_complicated_area.debug_constraints();
        let debug_file = opts
            .debug_output
            .unwrap_or_else(|| String::from("debug.txt"));
        most_complicated_area.debug_area(&mut dijk, debug_file, seed, 1000)?;
        most_complicated_area.print_area_to_file(output)?;
    } else if modus == 3 {
        let path_file = opts.path_file.unwrap();
        println!("loading file: {}", path_file.display());
        let paths: Vec<Path> = serde_yaml::from_reader(BufReader::new(File::open(path_file)?))?;
        println!("loaded {} paths", paths.len());

        let exact_start = Instant::now();
        let sum_ch = std::sync::atomic::AtomicUsize::new(0);

        let thread_count = num_cpus::get().min(paths.len());
        let item_per_thread = paths.len() / thread_count;

        let intersections = vec![vec![]; paths.len()];
        let mut path_plus_result: Vec<_> = paths.iter().zip(intersections.into_iter()).collect();

        crossbeam::scope(|scope| {
            let chunks = path_plus_result.chunks_mut(item_per_thread);
            for chunk in chunks {
                scope.spawn(|_| {
                    let mut dijk = dijk.clone();
                    for (p, intersection) in chunk {
                        let s = *p.nodes.first().unwrap();
                        let t = *p.nodes.last().unwrap();
                        let mut area_calculator = AreaCalculator::new(
                            s,
                            t,
                            p.total_dimension_costs.iter().map(|&v| v as u32).collect(),
                        );
                        area_calculator.calculate_area(&mut dijk, false);
                        *intersection = area_calculator.get_intersections();
                        sum_ch.fetch_add(
                            area_calculator.ch_counter,
                            std::sync::atomic::Ordering::Relaxed,
                        );
                        // println!("{}: {}", i, area_calculator.ch_counter);
                    }
                });
            }
        })
        .expect("There were threading errors");
        let exact_time = exact_start.elapsed();
        println!("exact spaces wall clock time: {}", exact_time.as_secs_f64());

        let intersections = path_plus_result.into_iter().map(|t| t.1).collect();
        print_intersections_to_file(output, intersections)?;
        println!(
            "Finished. Average CH calls: {}",
            sum_ch.into_inner() as f64 / paths.len() as f64
        );
    } else if modus == 4 {
        let nodes_dist = Uniform::from(0..dijk.graph.nodes.len() as u32);
        let s = nodes_dist.sample(&mut rng);
        let t = nodes_dist.sample(&mut rng);
        let all_areas = get_all_optimal_areas_as_string(&mut dijk, s, t);
        let mut file = File::create(output)?;
        file.write_all(all_areas.as_bytes())?;
    } else if modus == 5 {
        let nodes_dist = Uniform::from(0..dijk.graph.nodes.len() as u32);
        let s = nodes_dist.sample(&mut rng);
        let t = nodes_dist.sample(&mut rng);
        let all_paths = get_all_optimal_costs_as_string(&mut dijk, s, t);
        let mut file = File::create(output)?;
        file.write_all(all_paths.as_bytes())?;
    } else if modus == 6 {
        let path_file = opts.path_file.unwrap();
        println!("loading file: {}", path_file.display());
        let paths: Vec<Path> = serde_yaml::from_reader(BufReader::new(File::open(path_file)?))?;
        println!("loaded {} paths", paths.len());

        for i in 0..paths.len() {
            let s = paths[i].nodes.first().unwrap();
            let t = paths[i].nodes.last().unwrap();
            let mut area_calculator = AreaCalculator::new(
                *s,
                *t,
                paths[i]
                    .total_dimension_costs
                    .iter()
                    .map(|&v| v as u32)
                    .collect(),
            );
            area_calculator.calculate_area(&mut dijk, false);
            area_calculator
                .print_area_to_file(format!("path-{}.txt", i))
                .unwrap();
        }
    }
    Ok(())
}
