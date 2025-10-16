use std::{
    cmp::{Reverse, max, min},
    collections::{BinaryHeap, HashMap, HashSet},
    hash::Hash,
    i64,
    process::exit,
    sync::Arc,
    vec,
};

use rand::{self, Rng, SeedableRng, rng, rngs::SmallRng};
use rayon::prelude::*;

use crate::{Job, Optimize, Process, Spec, logger::print_genome};
use crate::{Stock, logger::Logger};

const _ELITISM_CNT: i32 = 2;
const TOP_PCT: f64 = 0.1;
const BOT_PCT: f64 = 0.2;
const HEAD_PCT: f64 = 0.5;
const MAX_POPULATION: usize = 400;
const PERCENT_CHANCE_TO_MUTATE: f64 = 3.0;
const MAX_CYCLES: i64 = 10000;
const DEBUG_WRITE_MODE: bool = true;
const RESET_VALUE_GEN: i64 = 2;
const RESET_DIVIDER: i64 = 8;
const MAX_RESET_VALUE: i64 = 20;
const ISLANDS_COUNT: usize = 8;
const MAX_POPULATION_PER_ISLAND: usize = MAX_POPULATION / ISLANDS_COUNT;

#[derive(Default)]
pub struct Population {
    candidates: [Vec<Genome>; ISLANDS_COUNT],
}

#[derive(Clone)]
pub struct Genome {
    pub keys: Vec<f64>,
    pub fitness: i64,
    pub pending_stock_divider: i32,
    pub spec: Arc<Spec>,
    pub disabled_processes: bool,
}

impl Genome {
    pub fn new(
        keys: Vec<f64>,
        fitness: i64,
        pending_stock_divider: i32,
        spec: Arc<Spec>,
        disabled_processes: bool,
    ) -> Self {
        Self {
            keys,
            fitness,
            pending_stock_divider,
            spec,
            disabled_processes,
        }
    }
}

impl Hash for Genome {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pending_stock_divider.hash(state);
        self.disabled_processes.hash(state);
        for &k in &self.keys {
            quantize(k, 1e6).hash(state);
        }
    }
}
impl PartialEq for Genome {
    fn eq(&self, other: &Self) -> bool {
        self.pending_stock_divider == other.pending_stock_divider
            && self.disabled_processes == other.disabled_processes
            && self
                .keys
                .iter()
                .zip(&other.keys)
                .all(|(a, b)| quantize(*a, 1e6) == quantize(*b, 1e6))
    }
}
impl Eq for Genome {}

pub struct Sim<'a> {
    spec: &'a Spec,
    time: i64,
    stocks: HashMap<String, i64>,
    running: BinaryHeap<Reverse<Job>>,
}

pub fn priority_from_keys(keys: &[f64]) -> Vec<usize> {
    let mut idx: Vec<usize> = (0..keys.len()).collect();
    idx.sort_by(|&i, &j| keys[i].total_cmp(&keys[j]));
    idx
}

fn quantize(x: f64, scale: f64) -> i64 {
    (x * scale).round() as i64
}

fn inputs_available(p: &Process, stocks: &HashMap<String, i64>) -> bool {
    p.needs
        .iter()
        .all(|n| stocks.get(&n.name).copied().unwrap_or(0) >= n.quantity)
}

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

pub fn eval_fitness(cand: &mut Genome, horizon: i64) -> (i64, Sim) {
    let order = priority_from_keys(&cand.keys);

    let mut s = Sim {
        spec: &cand.spec.as_ref(),
        time: 0,
        stocks: cand.spec.init_stocks.clone(),
        running: BinaryHeap::new(),
    };

    let mut pending: HashMap<String, i64> = HashMap::new();
    // let mut logger = Logger::new(&s.stocks, "stock_evolution.csv");

    let target = match &s.spec.optimize {
        Optimize::Quantity(name) | Optimize::Time(name) => name.as_str(),
    };

    while s.time < horizon {
        // if DEBUG_WRITE_MODE {
        //     if (logger) {
        //         logger.log_stocks(s.time, &s.stocks);
        //     }
        // }
        for (pos, &pid) in order.iter().enumerate() {
            if cand.keys[pos] == 1.0 {
                continue;
            }

            let p = &s.spec.processes[pid];

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
                finish_time: s.time + p.duration,
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
    cand.fitness = fit;
    (fit, s)
}

fn gen_pending_stock_divider() -> i32 {
    let dividers = vec![
        1, 2, 4, 6, 8, 10, 25, 50, 75, 100, 125, 150, 175, 200, 225, 250, 275, 300, 325, 350, 375,
        400, 425, 450, 475, 500,
    ];
    // let dividers = vec![2];
    dividers[rand::rng().random_range(0..dividers.len())]
}

fn gen_random_keys(n: usize) -> Vec<f64> {
    let mut random_keys: Vec<f64> = vec![];
    let mut rng = SmallRng::from_os_rng();
    for _ in 0..n {
        // random_keys.push(rng().random::<f64>());
        random_keys.push(rng.random::<f64>());
    }
    random_keys
    // vec![0.01, 0.02, 0.03, 0.04, 0.05, 0.06, 0.07, 0.08, 0.09, 0.1]
    // vec![0.1, 0.09, 0.08, 0.07, 0.06, 0.05, 0.04, 0.03, 0.02, 0.01]
}

pub fn disable_rdm_processes(keys: &mut Vec<f64>) -> bool {
    let mut r = rng();
    let chance_to_disable = r.random::<f64>();
    let disabled_processes = if chance_to_disable > 0.5 { true } else { false };
    if !disabled_processes {
        return disabled_processes;
    }

    let n_disabled_processes = r.random_range(0..=keys.len());
    for _ in 0..n_disabled_processes {
        let key = r.random_range(0..keys.len());
        keys[key] = 1.0;
    }
    disabled_processes
}

pub fn gen_random_genome(spec: Arc<Spec>, process_nbr: usize) -> Genome {
    let mut random_keys = gen_random_keys(process_nbr);
    let disabled_processes = disable_rdm_processes(&mut random_keys);
    disable_rdm_processes(&mut random_keys);
    let divider = gen_pending_stock_divider();

    // eprintln!("{:?}", wait_cycles);
    Genome::new(random_keys, 0, divider, spec.clone(), disabled_processes)
}

pub fn gen_initial_pop(spec: Arc<Spec>, process_nbr: usize) -> Population {
    let mut pop: Population = Default::default();
    for idx in 0..ISLANDS_COUNT {
        for _ in 0..MAX_POPULATION_PER_ISLAND {
            let cand = gen_random_genome(spec.clone(), process_nbr);
            pop.candidates[idx].push(cand);
        }
    }
    pop
}

fn _mutate(_cand: &mut Genome) {}

fn crossover(p1: &Genome, p2: &Genome) -> Genome {
    let mut keys: Vec<f64> = vec![];
    let divider: i32;
    let processes_len = p1.keys.len();
    for (k_n, k) in p1.keys.iter().enumerate() {
        let r = rng().random::<f64>();
        if r < HEAD_PCT {
            keys.push(p1.keys[k_n]);
        } else {
            keys.push(p2.keys[k_n]);
        }
    }

    let mut r = rng().random::<f64>();
    if r < HEAD_PCT {
        divider = p1.pending_stock_divider;
    } else {
        divider = p2.pending_stock_divider;
    }

    r = rng().random::<f64>();

    Genome::new(keys, 0, divider, p1.spec.clone(), p1.disabled_processes)
}

fn pick_parents<'a>(sorted: &'a [Genome], elite_cnt: usize) -> (&'a Genome, &'a Genome) {
    let mut r = rand::rng();
    let ec = elite_cnt.clamp(1, MAX_POPULATION_PER_ISLAND);

    let i_elite = r.random_range(0..ec);

    if ec < MAX_POPULATION_PER_ISLAND {
        let i_other = r.random_range(ec..MAX_POPULATION_PER_ISLAND);
        (&sorted[i_elite], &sorted[i_other])
    } else {
        let i_other = if MAX_POPULATION_PER_ISLAND > 1 {
            let mut j = r.random_range(0..MAX_POPULATION_PER_ISLAND);
            if j == i_elite {
                j = (j + 1) % MAX_POPULATION_PER_ISLAND;
            }
            j
        } else {
            0
        };
        // if rng().random::<f64>() > 0.5 {
        //     return (&sorted[i_elite], &sorted[i_other]);
        // } else {
        //     return (&sorted[i_other], &sorted[i_elite]);
        // }
        (&sorted[i_elite], &sorted[i_other])
    }
}

pub fn run_ga(mut pop: Population, generations: usize) -> Genome {
    let mut best_cands: Vec<Genome> = vec![];
    for idx in 0..ISLANDS_COUNT {
        for (_index, genome) in pop.candidates[idx].iter_mut().enumerate() {
            // eprintln!("p2  : {:?}", genome.wait_cycles);
            let _ = eval_fitness(genome, MAX_CYCLES);
            // eprintln!("Genome {} of generation has {} fitness.", index, fit);
        }
        best_cands.push(
            pop.candidates[idx]
                .iter()
                .max_by_key(|c| c.fitness)
                .unwrap()
                .clone(),
        );
    }

    let spec = best_cands[0].spec.clone();

    let mut last_improvment: Vec<i64> = vec![0; ISLANDS_COUNT];
    let mut best_fitness: Vec<i64> = vec![0; ISLANDS_COUNT];
    let mut current_reset_value: Vec<i64> = vec![RESET_VALUE_GEN; ISLANDS_COUNT];
    let mut last_wipe_improvments: Vec<bool> = vec![false; ISLANDS_COUNT];

    for _gen in 0..generations {
        for isl_idx in 0..ISLANDS_COUNT {
            if isl_idx == ISLANDS_COUNT - 1 {
                eprintln!(
                    "Best Genome of generation {} of island {} has {} fitness and divider : {} ",
                    _gen,
                    isl_idx,
                    best_cands[isl_idx].fitness,
                    best_cands[isl_idx].pending_stock_divider
                );
            }
            // print_genome(&best);
            pop.candidates[isl_idx].sort_by_key(|g| std::cmp::Reverse(g.fitness));

            let elite_cnt = (TOP_PCT * MAX_POPULATION_PER_ISLAND as f64) as usize;

            let bot_cnt = ((BOT_PCT * MAX_POPULATION_PER_ISLAND as f64).round() as usize)
                .clamp(1, MAX_POPULATION_PER_ISLAND);

            let survivors_end = MAX_POPULATION_PER_ISLAND
                .saturating_sub(bot_cnt)
                .max(elite_cnt);

            let mut next: Vec<Genome> = Vec::with_capacity(MAX_POPULATION_PER_ISLAND as usize);

            // eprintln!("gen: {}, l: {}", _gen, last_improvment[isl_idx]);
            if _gen as i64 - last_improvment[isl_idx] > current_reset_value[isl_idx] {
                eprintln!("we had to wipe them all but the best one");
                if last_wipe_improvments[isl_idx] == false {
                    current_reset_value[isl_idx] = min(
                        MAX_RESET_VALUE,
                        current_reset_value[isl_idx]
                            + _gen as i64 / (MAX_POPULATION_PER_ISLAND as i64 / RESET_DIVIDER),
                    );
                    // current_reset_value += 2;
                }
                last_improvment[isl_idx] = _gen as i64;
                last_wipe_improvments[isl_idx] = false;
                next.push(pop.candidates[isl_idx][0].clone());
                while next.len() < MAX_POPULATION_PER_ISLAND {
                    next.push(gen_random_genome(spec.clone(), spec.processes.len()));
                }
            } else {
                // we keep our percentages elites on this island
                next.extend(pop.candidates[isl_idx].iter().take(elite_cnt).cloned());

                // and we also take the best of the previous isl

                // if isl_idx == 0 {
                //     next.push(pop.candidates[ISLANDS_COUNT - 1][0].clone());
                // } else {
                //     next.push(pop.candidates[isl_idx - 1][0].clone());
                // }

                let mut seen: HashSet<Genome> = HashSet::with_capacity(MAX_POPULATION_PER_ISLAND);
                for g in &next {
                    seen.insert(g.clone());
                }

                if isl_idx == ISLANDS_COUNT - 1 {
                    for idx in 0..ISLANDS_COUNT {
                        let cand = pop.candidates[idx][0].clone();
                        if seen.insert(cand.clone()) {
                            next.push(cand);
                        }
                    }
                }

                // eprintln!("sE : {}", survivors_end);
                // eprintln!("next.len() : {}", next.len());
                // eprintln!("elite_cnt : {}", elite_cnt);

                while next.len() < survivors_end {
                    let (p1, p2) = pick_parents(&pop.candidates[isl_idx], elite_cnt);
                    let child = crossover(p1, p2);
                    // eprintln!(
                    // "p1 keys: {:?}, p2 keys: {:?}, child keys: {:?}",
                    // p1.keys, p2.keys, child.keys
                    // );
                    next.push(child);
                }

                while next.len() < MAX_POPULATION_PER_ISLAND {
                    next.push(gen_random_genome(spec.clone(), spec.processes.len()));
                }
            }

            pop.candidates[isl_idx] = next;

            // for (index, genome) in pop.candidates.iter_mut().enumerate() {
            //     let fit = eval_fitness(genome, MAX_CYCLES);
            //     // eprintln!("Genome {} of generation {_gen} has {}  fitness.", index, fit);
            // }

            pop.candidates[isl_idx]
                .par_iter_mut()
                .enumerate()
                .for_each(|(i, cand)| {
                    eval_fitness(cand, MAX_CYCLES);
                });

            if let Some(cur_best) = pop.candidates[isl_idx].iter().max_by_key(|c| c.fitness) {
                if cur_best.fitness > best_cands[isl_idx].fitness {
                    best_cands[isl_idx] = cur_best.clone();
                }
            }
            if best_cands[isl_idx].fitness > best_fitness[isl_idx] {
                last_improvment[isl_idx] = _gen as i64;
                best_fitness[isl_idx] = best_cands[isl_idx].fitness;
                last_wipe_improvments[isl_idx] = true;
            }

            // if isl_idx == ISLANDS_COUNT - 1 {
            //     let (_, s) = eval_fitness(&mut best_cands[isl_idx], MAX_CYCLES);
            //     eprintln!("stocks of best_cands[isl_idx] : {:?}", s.stocks);
            // }
        }
    }
    best_cands.sort_by_key(|g| std::cmp::Reverse(g.fitness));

    let (f, s) = eval_fitness(&mut best_cands[0], MAX_CYCLES);
    // let (f2, s2) = eval_fitness(&best_cands[ISLANDS_COUNT - 1].clone(), MAX_CYCLES);
    eprintln!(
        "fitness of best overall is {} and stocks of best overall : {:?}",
        f, s.stocks
    );
    // eprintln!(
    //     "fitness of best overall is {} and stocks of best overall : {:?}",
    //     f2, s2.stocks
    // );
    best_cands[0].clone()
}
