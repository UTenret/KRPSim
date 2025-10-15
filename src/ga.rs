use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
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
const TOP_PCT: f64 = 0.05;
const BOT_PCT: f64 = 0.2;
const HEAD_PCT: f64 = 0.7;
const MAX_POPULATION: usize = 40;
const PERCENT_CHANCE_TO_MUTATE: f64 = 3.0;
const MAX_CYCLES: i64 = 10000;
const DEBUG_WRITE_MODE: bool = true;

#[derive(Default)]
pub struct Population {
    candidates: Vec<Genome>,
}

#[derive(Clone)]
pub struct Genome {
    pub keys: Vec<f64>,
    pub fitness: i64,
    pub pending_stock_divider: i32,
    pub spec: Arc<Spec>,
    pub delay: bool,
    pub disabled_processes: bool,
    pub wait_cycles: Vec<i64>,
}

impl Genome {
    pub fn new(
        keys: Vec<f64>,
        fitness: i64,
        pending_stock_divider: i32,
        spec: Arc<Spec>,
        delay: bool,
        wait_cycles: Vec<i64>,
        disabled_processes: bool,
    ) -> Self {
        Self {
            keys,
            fitness,
            pending_stock_divider,
            spec,
            delay,
            wait_cycles,
            disabled_processes,
        }
    }
}

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

pub fn higher_priority_runnable_before_delay(
    s: &Sim,
    spec: &Spec,
    order: &[usize],
    pos: usize,
    delay: i64,
) -> bool {
    if delay <= 0 || pos == 0 {
        return false;
    }

    let time_limit = s.time + delay;

    let mut now = s.time;
    let mut stocks: HashMap<String, i64> = s.stocks.clone();
    let mut running = s.running.clone();

    let mut hp_can_run_now = |stocks: &HashMap<String, i64>, now: i64| -> bool {
        for &pid in &order[..pos] {
            let p = &spec.processes[pid];
            let dur = p.duration;
            if now + dur <= time_limit && inputs_available(p, stocks) {
                return true;
            }
        }
        false
    };

    if hp_can_run_now(&stocks, now) {
        return true;
    }

    loop {
        match running.peek() {
            Some(Reverse(top)) if top.finish_time <= time_limit => {
                let t_next = top.finish_time;
                now = t_next;

                while let Some(Reverse(job)) = running.peek() {
                    if job.finish_time != t_next {
                        break;
                    }
                    let Reverse(job) = running.pop().unwrap();
                    for r in &job.results {
                        *stocks.entry(r.name.clone()).or_insert(0) += r.quantity;
                    }
                }

                if hp_can_run_now(&stocks, now) {
                    return true;
                }
            }
            _ => break,
        }
    }
    false
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
            // let mut should_wait = false;

            // if cand.delay {
            //     if higher_priority_runnable_before_delay(
            //         &s,
            //         s.spec,
            //         &order,
            //         pos,
            //         cand.wait_cycles[pos],
            //     ) {
            //         should_wait = true;
            //     }
            // }

            // if should_wait {
            //     continue;
            // }

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
    rand::rng().random_range(1..500)
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

pub fn gen_wait_cycles(spec: Arc<Spec>, processes_len: usize) -> Vec<i64> {
    let mut wait_cycles = vec![0; processes_len];
    // todo refine delay ranges based on max delay, delays of processes etc

    let nbr_delayed = rng().random_range(1..=processes_len);

    let mut r = rng();
    let mut indices: Vec<usize> = (0..processes_len).collect();
    for i in 0..nbr_delayed {
        let j = r.random_range(i..processes_len);
        indices.swap(i, j);
    }

    for &idx in &indices[..nbr_delayed] {
        // todo FIX DELAY GENERATION
        // pick possible delay from delays of higher processes
        wait_cycles[idx] = r.random_range(1..=5) as i64;
    }

    wait_cycles
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
    // let delay_chance = rng().random::<f64>();
    // let delay: bool = if delay_chance > 0.5 { true } else { false };
    let delay: bool = true;
    let wait_cycles = if delay {
        gen_wait_cycles(spec.clone(), process_nbr)
    } else {
        vec![0; process_nbr]
    };
    // eprintln!("{:?}", wait_cycles);
    Genome::new(
        random_keys,
        0,
        divider,
        spec.clone(),
        delay,
        wait_cycles,
        disabled_processes,
    )
}

pub fn gen_initial_pop(spec: Arc<Spec>, process_nbr: usize) -> Population {
    let mut pop: Population = Default::default();
    for _ in 0..MAX_POPULATION {
        let cand = gen_random_genome(spec.clone(), process_nbr);
        pop.candidates.push(cand);
    }
    pop
}

fn _mutate(_cand: &mut Genome) {}

fn crossover(p1: &Genome, p2: &Genome) -> Genome {
    let mut keys: Vec<f64> = vec![];
    let divider: i32;
    let processes_len = p1.keys.len();
    let mut wait_cycles = vec![0; processes_len];
    let mut delay = false;
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

    if r < HEAD_PCT && p1.delay {
        delay = true;
        for index in 0..processes_len {
            let r = rng().random::<f64>();
            if r < HEAD_PCT {
                wait_cycles[index] = p1.wait_cycles[index];
            } else {
                wait_cycles[index] = p2.wait_cycles[index];
            }
        }
    } else if r > HEAD_PCT && p2.delay {
        delay = true;
        for index in 0..processes_len {
            let r = rng().random::<f64>();
            if r < HEAD_PCT {
                wait_cycles[index] = p2.wait_cycles[index];
            } else {
                wait_cycles[index] = p1.wait_cycles[index];
            }
        }
    }

    Genome::new(
        keys,
        0,
        divider,
        p1.spec.clone(),
        delay,
        wait_cycles,
        p1.disabled_processes,
    )
}

fn pick_parents<'a>(sorted: &'a [Genome], elite_cnt: usize) -> (&'a Genome, &'a Genome) {
    let mut r = rand::rng();
    let ec = elite_cnt.clamp(1, MAX_POPULATION);

    let i_elite = r.random_range(0..ec);

    if ec < MAX_POPULATION {
        let i_other = r.random_range(ec..MAX_POPULATION);
        (&sorted[i_elite], &sorted[i_other])
    } else {
        let i_other = if MAX_POPULATION > 1 {
            let mut j = r.random_range(0..MAX_POPULATION);
            if j == i_elite {
                j = (j + 1) % MAX_POPULATION;
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
    for (_index, genome) in pop.candidates.iter_mut().enumerate() {
        // eprintln!("p2  : {:?}", genome.wait_cycles);
        let _ = eval_fitness(genome, MAX_CYCLES);
        // eprintln!("Genome {} of generation has {} fitness.", index, fit);
    }

    let mut best = pop
        .candidates
        .iter()
        .max_by_key(|c| c.fitness)
        .unwrap()
        .clone();

    let spec = best.spec.clone();

    for _gen in 0..generations {
        // eprintln!(
        //     "Best Genome of generation {} has {} fitness and delay : {}, wait_cycles : {:?}",
        //     _gen, best.fitness, best.delay, best.wait_cycles
        // );
        print_genome(&best);
        pop.candidates.sort_by_key(|g| std::cmp::Reverse(g.fitness));

        let elite_cnt = (TOP_PCT * MAX_POPULATION as f64) as usize;

        let bot_cnt = ((BOT_PCT * MAX_POPULATION as f64).round() as usize).clamp(1, MAX_POPULATION);

        let survivors_end = MAX_POPULATION.saturating_sub(bot_cnt).max(elite_cnt);

        let mut next: Vec<Genome> = Vec::with_capacity(MAX_POPULATION as usize);

        next.extend(pop.candidates.iter().take(elite_cnt).cloned());

        // eprintln!("sE : {}", survivors_end);
        // eprintln!("next.len() : {}", next.len());
        // eprintln!("elite_cnt : {}", elite_cnt);

        while next.len() < survivors_end {
            let (p1, p2) = pick_parents(&pop.candidates, elite_cnt);
            let child = crossover(p1, p2);
            // eprintln!(
            // "p1 keys: {:?}, p2 keys: {:?}, child keys: {:?}",
            // p1.keys, p2.keys, child.keys
            // );
            next.push(child);
        }

        while next.len() < MAX_POPULATION {
            next.push(gen_random_genome(spec.clone(), spec.processes.len()));
        }

        pop.candidates = next;

        // for (index, genome) in pop.candidates.iter_mut().enumerate() {
        //     let fit = eval_fitness(genome, MAX_CYCLES);
        //     // eprintln!("Genome {} of generation {_gen} has {} fitness.", index, fit);
        // }

        pop.candidates
            .par_iter_mut()
            .enumerate()
            .for_each(|(i, cand)| {
                eval_fitness(cand, MAX_CYCLES);
            });

        if let Some(cur_best) = pop.candidates.iter().max_by_key(|c| c.fitness) {
            if cur_best.fitness > best.fitness {
                best = cur_best.clone();
            }
        }
        let (_, s) = eval_fitness(&mut best, MAX_CYCLES);
        eprintln!("stocks of best : {:?}", s.stocks);
    }

    let (_, s) = eval_fitness(&mut best, MAX_CYCLES);
    eprintln!("stocks of best overall : {:?}", s.stocks);
    best
}
