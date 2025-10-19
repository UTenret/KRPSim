use std::collections::HashMap;
use std::env;
use std::fs;
use std::hash::Hash;
use std::os::unix::process;
use std::process::exit;
mod ga;
mod logger;
mod parser;
use std::cmp::Ordering;
use std::sync::Arc;

use rand::Rng;
use rand::rng;
use rayon::iter;

use crate::ga::gen_initial_pop;
use crate::ga::run_ga;

#[derive(Debug, Clone)]
pub enum Optimize {
    Time(String),
    Quantity(String),
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Stock {
    name: String,
    quantity: i64,
}

impl Stock {
    pub fn new(name: &str, quantity: i64) -> Self {
        Self {
            name: name.to_string(),
            quantity,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Job {
    finish_time: i64,
    proc_id: usize,
}

impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.finish_time == other.finish_time && self.proc_id == other.proc_id
    }
}
impl Eq for Job {}

impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.finish_time.cmp(&other.finish_time) {
            Ordering::Equal => self.proc_id.cmp(&other.proc_id),
            ord => ord,
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct Process {
    id: usize,
    name: String,
    needs: Vec<Stock>,
    results: Vec<Stock>,
    duration: i64,
}

impl Process {
    fn new(id: usize, name: &str, needs: Vec<Stock>, results: Vec<Stock>, duration: i64) -> Self {
        Self {
            id,
            name: name.to_string(),
            needs,
            results,
            duration,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Spec {
    processes: Vec<Process>,
    init_stocks: HashMap<String, i64>,
    optimize: Optimize,
}

impl Spec {
    fn new(processes: Vec<Process>, init_stocks: HashMap<String, i64>, optimize: Optimize) -> Self {
        Self {
            processes,
            init_stocks,
            optimize,
        }
    }
}

/*
optimized structure for simulation
it's a little bit less readable for it is faster
*/
#[derive(Debug)]
pub struct SimSpec {
    needs: Vec<Vec<(usize, i64)>>, // id/qty
    results: Vec<Vec<(usize, i64)>>,
    durations: Vec<i64>,
    init_stocks: Vec<i64>, // the idx is the id of the stock
    optimize: Optimize,
    target_stock_id: usize,
}

impl SimSpec {
    fn from_spec(spec: &Spec) -> Self {
        eprintln!(
            "{:?}",
            spec.processes
                .iter()
                .map(|p| p
                    .needs
                    .iter()
                    .map(|s| (&s.name, s.quantity))
                    .collect::<Vec<_>>())
                .collect::<Vec<_>>()
        );

        let target = match &spec.optimize {
            Optimize::Quantity(name) | Optimize::Time(name) => name.as_str(),
        };

        let mut target_stock_id = 0;

        /*
        building this just to get an index/id for each stock, consistent for this scope for needs and results below
        we dont need the order to stay the same on every program start, which it is not going to be
        as order in a hashmap is not guaranteed
        however it will remain consistent for the remainder of the execution of the program
        as long as we only instanciate this once
         */
        let init_stocks_name_to_id: HashMap<String, usize> = spec
            .init_stocks
            .iter()
            .enumerate()
            .map(|(idx, p)| {
                if p.0 == target {
                    target_stock_id = idx;
                }
                (p.0.clone(), idx)
            })
            .collect();

        let init_stocks: Vec<i64> = spec.init_stocks.iter().map(|s| *s.1).collect();

        let build_vec = |p: &Vec<Stock>| {
            p.iter()
                .map(|s| (init_stocks_name_to_id[&s.name], s.quantity))
                .collect()
        };

        let needs: Vec<Vec<(usize, i64)>> =
            spec.processes.iter().map(|p| build_vec(&p.needs)).collect();

        let results: Vec<Vec<(usize, i64)>> = spec
            .processes
            .iter()
            .map(|p| build_vec(&p.results))
            .collect();

        let durations = spec.processes.iter().map(|p| p.duration).collect();

        eprintln!("results : {:?}", results);
        eprintln!("results : {:?}", needs);

        Self {
            needs,
            results,
            durations,
            init_stocks,
            optimize: spec.optimize.clone(),
            target_stock_id,
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage : cargo run --release -- input_file_path [optional:--seed=<12345>] ");
        std::process::exit(0);
    }

    let file_path = &args[1];
    let seed: i64 = if args.len() > 2 {
        args[2].parse().expect("Not a valid seed number")
    } else {
        rng().random()
    };

    let contents = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read the contents of the file : {}", e);
        exit(1);
    });

    let spec = parser::parse_spec(&contents).unwrap_or_else(|e| {
        eprintln!("Error while parsing the contents of the file : {}", e);
        exit(1);
    });

    if spec.processes.is_empty() {
        eprintln!("No process worth starting!");
    }

    let sim_spec = Arc::from(SimSpec::from_spec(&spec));

    // println!("spec: {:?}", spec);

    let pop = gen_initial_pop(spec.processes.len());
    let best = run_ga(sim_spec, pop, 100);

    // println!("sim_spec: {:?}", sim_spec);

    eprintln!("Best genome has {} fitness", best.fitness);
}
