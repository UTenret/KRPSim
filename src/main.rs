use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::exit;
mod ga;
mod parser;
mod logger;
use std::cmp::Ordering;
use std::sync::Arc;

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
    results: Vec<Stock>,
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
    delay: i64,
}

impl Process {
    fn new(id: usize, name: &str, needs: Vec<Stock>, results: Vec<Stock>, delay: i64) -> Self {
        Self {
            id,
            name: name.to_string(),
            needs,
            results,
            delay,
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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("You need to specify a file");
        std::process::exit(0);
    }

    let file_path = &args[1];

    let contents = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read the contents of the file : {}", e);
        exit(1);
    });

    let spec = parser::parse_spec(&contents).unwrap_or_else(|e| {
        eprintln!("Error while parsing the contents of the file : {}", e);
        exit(1);
    });

    if spec.processes.is_empty() {
        println!("No process worth starting!");
    }

    println!("spec: {:?}", spec);

    let spec_arc = Arc::new(spec.clone());
    let pop = gen_initial_pop(spec_arc, spec.processes.len());

    let best = run_ga(pop, 200);

    eprintln!("Best genome has {} fitness", best.fitness);

    // let mut max_score = 0;
    // for _ in 1..50 {
    //     let score = random_solve(spec.clone());
    //     max_score = max(score, max_score);
    //     println!("score: {}", score);
    // }
    // println!("max_score: {}", max_score);

    // let possible_actions = Vec

    // println!("File contents : {}", contents.unwrap());

    // let mut cycle: i32 = 0;

    // while (cycle < 2000) {
    //     cycle += 1;
    // }
}

/*

priority order
but
only start process
if outcome is needed by higher priority process
and
also only start process
if you will have enough resources after


and randomly block some process(they should never run)



wait time per process

before that, crossover

we take the top 15% as is
we crossover top 15% with bottom 85% to create
top% - bot%(new random) == 65%

we take some process from the top 15%, cross it with



cut one or more providers

then cut every useless gainer


for the stock you need to optimize

find the list of processes that produce it

2 scenarii : 

1. You have only one process capable of producing the thing we're trying to optimize
2. You have multiple processes


basically the idea is to look at needs of process(sses)
that produce the stock we need
check 



*/
