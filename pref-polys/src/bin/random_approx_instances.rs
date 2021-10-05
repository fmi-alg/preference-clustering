use std::{
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
    time::Instant,
};

use structopt::StructOpt;

use pref_polys::{
    graph::{
        self,
        dijkstra::{self, Dijkstra},
        path::Path,
    },
    preference::{self, ApproxPoint},
};
use pref_polys::{preference::SizeApproximation, utils::randomized_preference};
use pref_polys::{
    preference::{dir_iter, SetPreferences},
    utils::Preference,
};

use anyhow::{Context, Result};
use rand::rngs::StdRng;
use rand::{
    distributions::{Distribution, Uniform},
    prelude::SliceRandom,
};
use rand::{thread_rng, RngCore, SeedableRng};
use serde::{Deserialize, Serialize};

use std::convert::TryInto;

#[derive(StructOpt, Serialize, Deserialize)]
struct Opts {
    graph: Option<PathBuf>,
    #[structopt(short = "p", long)]
    num_paths: Option<usize>,
    #[structopt(short = "n", long)]
    num_prefs: Option<usize>,
    #[structopt(short = "s", long)]
    seed: Option<u64>,
    /// Approximation Strategy to use. Possible values are: axis, random, rotation
    #[structopt(short = "a", long)]
    approx: Option<ApproxStrategy>,
    /// Changes the number of directions to approximate in (only applicable with random and rotation)
    #[structopt(short = "c", long)]
    approx_count: Option<usize>,
    // Path to write result to
    #[structopt(short = "o", long, default_value = ".")]
    output_path: PathBuf,
    #[structopt(short = "f", long)]
    paths_file: Option<PathBuf>,
    #[structopt(long)]
    config_file: Option<PathBuf>,
    #[structopt(long)]
    config_only: bool,
}

fn main() -> Result<()> {
    let opts = handle_options()?;
    let seed = opts.seed.unwrap();
    let graph_file = opts.graph.as_ref().unwrap_or_else(|| {
        println!("A graph file is necessary for instance generation");
        Opts::clap().print_help().unwrap();
        std::process::exit(1);
    });
    if opts.approx.is_none() {
        println!("An approximation stratgey (-a) is necessary for instance generation");
        Opts::clap().print_help().unwrap();
        std::process::exit(1);
    }

    let graph = graph::parse_minimal_graph_file(graph_file)?;
    let mut dij = graph::dijkstra::Dijkstra::new(&graph);

    println!("using seed {}", seed);
    let mut rng = StdRng::seed_from_u64(seed);

    let path_start = Instant::now();
    let paths = create_paths(&opts, &mut dij, &mut rng);
    let path_time = path_start.elapsed();

    println!("path finding time: {}", path_time.as_secs_f64());
    println!("{} paths found", paths.len());

    let file = create_output_file(&opts, "paths.yml")?;
    serde_yaml::to_writer(file, &paths).context("Failed writing paths")?;

    dijkstra::TimeReports::dijkstra();
    dijkstra::TimeReports::clear_dijkstra_time();

    let mut set_pref = SetPreferences::new(dij, &paths)?;

    let approx_start = Instant::now();
    let approx = run_approximation(
        &opts,
        graph.dim.try_into().unwrap(),
        &mut set_pref,
        &mut rng,
    );
    let approx_time = approx_start.elapsed();

    println!(
        "approximation wall clock time: {}",
        approx_time.as_secs_f64()
    );
    preference::TimeReports::approximate_pref_spaces();
    dijkstra::TimeReports::dijkstra();

    let mut file = create_output_file(&opts, "inner.space")?;

    writeln!(file, "{}", approx.len())?;
    for a in &approx {
        let approx_points = ApproxPoint::inner_from_size_approximation(a);
        write!(file, "{}", approx_points.len())?;
        for v in approx_points.iter().flat_map(|a| &a.constraints).flatten() {
            write!(file, " {}", v)?;
        }
        writeln!(file)?;
    }
    file.flush()?;

    let mut file = create_output_file(&opts, "outer.space")?;

    writeln!(file, "{}", approx.len())?;
    for a in approx {
        let approx_points = ApproxPoint::outer_from_size_approximation(&a);
        write!(file, "{}", approx_points.len())?;
        for v in approx_points.iter().flat_map(|a| &a.constraints).flatten() {
            write!(file, " {}", v)?;
        }
        writeln!(file)?;
    }

    Ok(())
}

fn handle_options() -> Result<Opts> {
    let mut opts = Opts::from_args();
    let mut rng = thread_rng();
    let seed = opts.seed.unwrap_or_else(|| rng.next_u64());
    opts.seed = Some(seed);
    if let Some(ref config_file) = opts.config_file {
        opts = serde_yaml::from_reader(BufReader::new(
            File::open(config_file)
                .with_context(|| format!("failed to open {}", config_file.display()))?,
        ))?;
        if opts.seed.is_none() {
            opts.seed = Some(seed);
        }
    } else {
        opts.graph = opts.graph.and_then(|g| g.canonicalize().ok());
        let file = create_output_file(&opts, "config.yml")?;
        serde_yaml::to_writer(file, &opts).context("Failed writing paths")?;
        if opts.config_only {
            println!("Exiting after writing config.");
            std::process::exit(0);
        }
    }

    Ok(opts)
}

fn create_output_file(opts: &Opts, filename: &str) -> Result<BufWriter<File>> {
    let mut path = opts.output_path.clone();
    path.push(filename);
    let inner = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .with_context(|| format!("Trying to create file {}", path.display()))?;
    let file = BufWriter::new(inner);
    println!("writing {}", path.display());
    Ok(file)
}

fn create_paths(opts: &Opts, dij: &mut Dijkstra, rng: &mut StdRng) -> Vec<Path> {
    let mut paths: Vec<Path> = Vec::new();

    if let Some(num_paths) = opts.num_paths {
        let mut preferences = Vec::new();

        let num_prefs = opts.num_prefs.unwrap_or(num_paths);
        for _ in 0..num_prefs {
            preferences.push(randomized_preference(
                rng,
                dij.graph.dim.try_into().unwrap(),
            ));
        }

        let nodes_dist = Uniform::from(0..dij.graph.nodes.len() as u32);
        while paths.len() < num_paths {
            let s = nodes_dist.sample(rng);
            let t = nodes_dist.sample(rng);
            if s == t {
                continue;
            }
            let pref = if opts.num_paths.is_some() {
                preferences.choose(rng).unwrap()
            } else {
                &preferences[paths.len()]
            };

            if let Some(path) = graph::dijkstra::find_shortest_path(dij, &[s, t], pref) {
                paths.push(path);
            }
        }
    } else if let Some(ref path) = opts.paths_file {
        let file = BufReader::new(
            File::open(path)
                .with_context(|| format!("Trying to open paths file: {}", path.display()))
                .unwrap(),
        );
        paths = serde_yaml::from_reader(file)
            .context("Trying to read paths.")
            .unwrap();
    }
    paths
}

fn run_approximation(
    opts: &Opts,
    dim: usize,
    set_pref: &mut SetPreferences,
    rng: &mut StdRng,
) -> Vec<SizeApproximation> {
    let dim = dim - 1;
    println!("starting approximation");
    match opts.approx.as_ref().unwrap() {
        ApproxStrategy::Axis => {
            let iter = axis_iter(dim);
            set_pref
                .approximate_pref_spaces(iter)
                .expect("error when approximating")
        }
        ApproxStrategy::Random => {
            let directions = random_directions(
                dim,
                opts.approx_count
                    .expect("approx_count needed for 'random' approximation"),
                rng,
            );
            set_pref
                .approximate_pref_spaces(directions.iter().cloned())
                .expect("error when approximating")
        }
        ApproxStrategy::Rotation => {
            let iter = dir_iter(
                dim,
                opts.approx_count
                    .expect("approx_count needed for 'rotation' approximation"),
            );
            set_pref
                .approximate_pref_spaces(iter)
                .expect("error when approximating")
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
enum ApproxStrategy {
    Axis,
    Random,
    Rotation,
}

impl std::str::FromStr for ApproxStrategy {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "axis" => Ok(ApproxStrategy::Axis),
            "random" => Ok(ApproxStrategy::Random),
            "rotation" => Ok(ApproxStrategy::Rotation),
            _ => Err(format!(
                "Could not match any Approximation Strategy in: {}",
                s
            )),
        }
    }
}

fn axis_iter(dim: usize) -> impl Iterator<Item = Preference> + Clone {
    (0..dim).flat_map(move |i| {
        let mut pos: Preference = vec![0.0; dim].into();
        pos[i] = 1.0;
        let mut neg = pos.clone();
        neg[i] = -1.0;
        vec![pos, neg]
    })
}

fn random_directions(dim: usize, n: usize, rng: &mut StdRng) -> Vec<Preference> {
    let mut res = Vec::new();
    for _ in 0..n {
        let mut pref = randomized_preference(rng, dim);
        res.push(pref.clone());
        pref.iter_mut().for_each(|c| *c *= -1.0);
        res.push(pref);
    }
    res
}
