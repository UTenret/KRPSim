use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    i64,
    sync::Arc,
    vec,
};

use rand::{self, Rng, rng, seq::SliceRandom};

use crate::{Job, Optimize, Process, Spec};

const ELITISM_CNT: i32 = 3;
const MAX_POPULATION: i32 = 100;
const PERCENT_CHANCE_TO_MUTATE: f64 = 3.0;
const MAX_CYCLES: i64 = 25000;

#[derive(Default)]
pub struct Population {
    candidates: Vec<Genome>,
}

#[derive(Clone)]
pub struct Genome {
    keys: Vec<f64>,
    pub fitness: i64,
    pending_stock_divider: i32,
    spec: Arc<Spec>,
}

impl Genome {
    pub fn new(keys: Vec<f64>, fitness: i64, pending_stock_divider: i32, spec: Arc<Spec>) -> Self {
        Self {
            keys,
            fitness,
            pending_stock_divider,
            spec,
        }
    }
}

pub struct Sim<'a> {
    spec: &'a Spec,
    time: i64,
    stocks: HashMap<String, i64>,
    running: BinaryHeap<Reverse<Job>>,
}

fn priority_from_keys(keys: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..keys.len()).collect();
    idx.sort_by(|&i, &j| keys[i].total_cmp(&keys[j]));
    idx
}

fn inputs_available(p: &Process, stocks: &HashMap<String, i64>) -> bool {
    p.needs
        .iter()
        .all(|n| stocks.get(&n.name).copied().unwrap_or(0) >= n.quantity)
}

fn mutate() {}

fn crossover() {}

fn deficits_for_higher_priority(
    order: &[usize],
    pos: usize,
    spec: &Spec,
    stocks: &HashMap<String, i64>,
) -> HashMap<String, i64> {
    let mut def = HashMap::new();

    for (pidx, &hp_idx) in order[..pos].iter().enumerate() {
        let p = &spec.processes[hp_idx];

        // eprintln!("pidx: {}", pidx);
        if pidx == 0 {
            for r in &p.results {
                def.insert(r.name.clone(), i64::MAX);
            }
            if let Optimize::Quantity(_) = spec.optimize {
                for n in &p.needs {
                    def.insert(n.name.clone(), i64::MAX);
                }
            }
        }

        let blocked_by_inventory = p
            .needs
            .iter()
            .any(|n| stocks.get(&n.name).copied().unwrap_or(0) < n.quantity);
        if !blocked_by_inventory {
            continue;
        }

        for n in &p.needs {
            let have = stocks.get(&n.name).copied().unwrap_or(0);
            let deficit = n.quantity - have;
            if deficit > 0 {
                def.entry(n.name.clone())
                    .and_modify(|v: &mut i64| {
                        if *v != i64::MAX {
                            *v = v.saturating_add(deficit);
                        }
                    })
                    .or_insert(deficit);
            }
        }
    }
    // eprintln!("current def : {:?}", def);
    def
}

pub fn eval_fitness(cand: &mut Genome, horizon: i64) -> i64 {
    let order = priority_from_keys(&cand.keys);

    let mut s = Sim {
        spec: &cand.spec.as_ref(),
        time: 0,
        stocks: cand.spec.init_stocks.clone(),
        running: BinaryHeap::new(),
    };

    let mut pending: HashMap<String, i64> = HashMap::new();

    let target = match &s.spec.optimize {
        Optimize::Quantity(name) | Optimize::Time(name) => name.as_str(),
    };

    while s.time < horizon {
        for (pos, &pid) in order.iter().enumerate() {
            let p = &s.spec.processes[pid];
            // eprintln!("pos: {}, pid: {}, pname: {}", pos, pid, p.name);
            // eprintln!("current_stocks: {:?}", s.stocks);

            if !inputs_available(p, &s.stocks) {
                continue;
            }

            let deficit = deficits_for_higher_priority(&order, pos + 1, &s.spec, &s.stocks);
            let should_run = p.results.iter().any(|r| {
                deficit.get(&r.name).copied().unwrap_or(0)
                    > (pending.get(&r.name).copied().unwrap_or(0)
                        / cand.pending_stock_divider as i64)
            });

            if !should_run || p.results.is_empty() {
                continue;
            }

            for n in &p.needs {
                *s.stocks.entry(n.name.clone()).or_insert(0) -= n.quantity;
            }

            s.running.push(Reverse(Job {
                finish_time: s.time + p.delay,
                proc_id: pid,
                results: p.results.clone(),
            }));

            for r in &p.results {
                *pending.entry(r.name.clone()).or_insert(0) += r.quantity;
            }
        }

        if let Some(Reverse(top)) = s.running.peek() {
            let t_next = top.finish_time;

            s.time = t_next;

            while let Some(Reverse(job)) = s.running.peek() {
                if job.finish_time != t_next {
                    break;
                }
                let Reverse(job) = s.running.pop().unwrap();

                for r in job.results.iter() {
                    *s.stocks.entry(r.name.clone()).or_insert(0) += r.quantity;
                    let e = pending.entry(r.name.clone()).or_insert(0);
                    *e -= r.quantity;
                    if *e <= 0 {
                        pending.remove(&r.name);
                    }
                }
            }
        } else {
            break;
        }
    }

    let fit = *s.stocks.get(target).unwrap_or(&0);
    // eprintln!("{:?}", s.stocks);
    // eprintln!("target : {}", target);
    cand.fitness = fit;
    fit
}

fn gen_pending_stock_divider() -> i32 {
    rand::rng().random_range(1..500)
}

fn gen_random_keys(n: usize) -> Vec<f64> {
    let mut random_keys: Vec<f64> = vec![];
    for _ in 0..n {
        random_keys.push(rng().random::<f64>());
    }
    random_keys
}

pub fn gen_initial_pop(spec: Arc<Spec>, process_nbr: usize) -> Population {
    let mut pop: Population = Default::default();
    for _ in 0..MAX_POPULATION {
        let random_keys = gen_random_keys(process_nbr);
        let divider = gen_pending_stock_divider();
        println!("prio_process_q: {:?}", random_keys);
        let cand: Genome = Genome::new(random_keys, 0, divider, spec.clone());
        pop.candidates.push(cand);
    }
    pop
}

pub fn run_ga(mut pop: Population) -> Genome {
    for (index, genome) in pop.candidates.iter_mut().enumerate() {
        let fit = eval_fitness(genome, MAX_CYCLES);
        eprintln!("Genome {} has {} fitness.", index, fit);
    }

    let mut best = pop
        .candidates
        .iter()
        .max_by_key(|c| c.fitness)
        .unwrap()
        .clone();

    best
}
