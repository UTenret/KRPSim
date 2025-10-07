use rand::Rng;
use std::collections::BinaryHeap;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::process::exit;
mod parser;
use std::cmp::{Ordering, Reverse};

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
struct Job {
    cycles: i64,
    stock: Stock,
}

impl Eq for Job {}
impl PartialEq for Job {
    fn eq(&self, other: &Self) -> bool {
        self.cycles == other.cycles && self.stock.name == other.stock.name
    }
}
impl Ord for Job {
    fn cmp(&self, other: &Self) -> Ordering {
        self.cycles
            .cmp(&other.cycles)
            .then_with(|| self.stock.name.cmp(&other.stock.name))
    }
}
impl PartialOrd for Job {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Default, Debug, Clone)]
pub struct Process {
    name: String,
    needs: Vec<Stock>,
    results: Vec<Stock>,
    delay: i64,
}

impl Process {
    fn new(name: &str, needs: Vec<Stock>, results: Vec<Stock>, delay: i64) -> Self {
        Self {
            name: name.to_string(),
            needs,
            results,
            delay,
        }
    }
}

#[derive(Debug)]
pub struct Spec {
    stocks: HashMap<String, i64>,
    processes: Vec<Process>,
    optimize: Optimize,
}

impl Spec {
    fn new(processes: Vec<Process>, stocks: HashMap<String, i64>, optimize: Optimize) -> Self {
        Self {
            processes,
            stocks,
            optimize,
        }
    }
}

fn gen_possible_actions(spec: &Spec) -> Vec<Process> {
    let mut possible_actions: Vec<Process> = vec![];
    for process in &spec.processes {
        let mut can = true;
        for need in &process.needs {
            if spec.stocks[&need.name] < need.quantity {
                can = false;
                break;
            }
        }
        if can {
            possible_actions.push(process.clone());
        }
    }
    possible_actions
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

    let mut spec = parser::parse_spec(&contents).unwrap_or_else(|e| {
        eprintln!("Error while parsing the contents of the file : {}", e);
        exit(1);
    });

    println!("spec: {:?}", spec);

    let mut heap: BinaryHeap<Reverse<Job>> = BinaryHeap::new();

    for cycles in 0..2000 {
        while let Some(top) = heap.peek() {
            if top.0.cycles == cycles {
                let Reverse(job) = heap.pop().unwrap();
                *spec.stocks.entry(job.stock.name.clone()).or_insert(0) += job.stock.quantity;
            } else {
                break;
            }
        }

        let mut possible_actions: Vec<Process> = gen_possible_actions(&spec);

        while !possible_actions.is_empty() {
            let mut rng = rand::rng();
            let idx = rng.random_range(0..possible_actions.len());

            let action = &possible_actions[idx];

            println!("{}:{}", cycles, action.name);
            for stock in &action.needs {
                *spec.stocks.entry(stock.name.clone()).or_insert(0) -= stock.quantity;
            }

            for stock in &action.results {
                heap.push(Reverse(Job {
                    cycles: action.delay + cycles,
                    stock: stock.clone(),
                }));
            }
            possible_actions = gen_possible_actions(&spec);
        }
    }

    for stock in &spec.stocks {
        println!("{} => {}", stock.0, stock.1);
    }

    // let possible_actions = Vec

    // println!("File contents : {}", contents.unwrap());

    // let mut cycle: i32 = 0;

    // while (cycle < 2000) {
    //     cycle += 1;
    // }
}
