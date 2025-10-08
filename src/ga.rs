use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    i64, vec,
};

use rand::{seq::SliceRandom, thread_rng};

use crate::{Job, Optimize, Spec, Stock};

const ELITISM_CNT: i32 = 3;
const MAX_POPULATION: i32 = 100;
const PERCENT_CHANCE_TO_MUTATE: f64 = 3.0;

#[derive(Default)]
pub struct Population {
    candidates: Vec<Genome>,
}

pub struct Genome {
    priority_process_q: Vec<usize>,
    current_needs: Vec<HashMap<String, i64>>,
    fitness: i32,
    spec: Spec, // max_job_cnt_perc_to_total_jobs: Vec<Vec<f64>>,
}

impl Genome {
    pub fn new(
        priority_process_q: Vec<usize>,
        current_needs: Vec<HashMap<String, i64>>,
        fitness: i32,
        spec: Spec,
        //max_job_cnt_perc_to_total_jobs: Vec<Vec<f64>>,
    ) -> Self {
        Self {
            priority_process_q,
            current_needs,
            fitness,
            spec, // max_job_cnt_perc_to_total_jobs,
        }
    }
}

fn mutate() {}

fn crossover() {}

fn calc_fitness(pop: &mut Population) -> i64 {
    let mut genome_nbr = 0;
    for cand in &mut pop.candidates {
        let mut heap: BinaryHeap<Reverse<Job>> = BinaryHeap::new();
        let mut time = 50000;

        let mut needs_maps: Vec<HashMap<String, i64>> = gen_current_needs(cand);

        for cycles in 0..time {
            while let Some(top) = heap.peek() {
                if top.0.cycles == cycles {
                    let Reverse(job) = heap.pop().unwrap();
                    *cand.spec.stocks.entry(job.stock.name.clone()).or_insert(0) +=
                        job.stock.quantity;
                } else {
                    break;
                }
            }

            for (step_idx, &proc_idx) in cand.priority_process_q.iter().enumerate() {
                // needs_maps = gen_current_needs(cand);
                // 1) Can we run? (enough stocks for all needs)
                let can_run = cand.spec.processes[proc_idx].needs.iter().all(|need| {
                    let have = cand.spec.stocks.get(&need.name).copied().unwrap_or(0);
                    have >= need.quantity
                });
                if !can_run {
                    continue;
                }

                // 2) Should we run? (any result is currently needed at this step)
                let should_run = cand.spec.processes[proc_idx]
                    .results
                    .iter()
                    .any(|res| needs_maps[step_idx].get(&res.name).copied().unwrap_or(0) > 0);
                if !should_run || cand.spec.processes[proc_idx].results.is_empty() {
                    continue;
                }

                // 3) Consume needs now
                for need in &cand.spec.processes[proc_idx].needs {
                    *cand.spec.stocks.entry(need.name.clone()).or_insert(0) -= need.quantity;
                }

                for stock in &cand.spec.processes[proc_idx].results {
                    heap.push(Reverse(Job {
                        cycles: cand.spec.processes[proc_idx].delay + cycles,
                        stock: stock.clone(),
                    }));
                }
            }

            if heap.is_empty() {
                time = cycles;
                break;
            }
        }
        let looking_for = match &cand.spec.optimize {
            Optimize::Time(s) | Optimize::Quantity(s) => s.as_str(),
        };
        for stock in &cand.spec.stocks {
            if stock.0 == looking_for {
                println!(
                    "Genome {} => fitness {} for stock: {} with time: {}",
                    genome_nbr, stock.1, stock.0, time
                );
            }
        }
        genome_nbr += 1;
    }
    0
}

fn gen_rand_prio_process_q(n: usize) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    v.shuffle(&mut rand::rng());
    v
}

/*

needs for max prio is itself
otherwise going down its the list of stock needed by previous + previous itself

*/

fn gen_current_needs(cand: &Genome) -> Vec<HashMap<String, i64>> {
    let n = cand.spec.processes.len();
    let mut current_needs: Vec<HashMap<String, i64>> = Vec::with_capacity(n);
    current_needs.resize_with(n, HashMap::new);
    // first prio has itself

    let mut prev_proc_idx: usize = 0;
    for (c_needs_idx, &proc_idx) in cand.priority_process_q.iter().enumerate() {
        if c_needs_idx == 0 {
            for final_prod in &cand.spec.processes[*&proc_idx].results {
                current_needs[c_needs_idx].insert(final_prod.name.clone(), i64::MAX);
            }
            continue;
        }

        // eprintln!("{}", c_needs_idx);
        current_needs[c_needs_idx] = current_needs[c_needs_idx - 1].clone();

        for need in &cand.spec.processes[prev_proc_idx].needs {
            let have = cand.spec.stocks.get(&need.name).copied().unwrap_or(0);
            let deficit = need.quantity - have;
            if deficit > 0 {
                current_needs[c_needs_idx]
                    .entry(need.name.clone())
                    .and_modify(|v| {
                        if *v != i64::MAX {
                            *v += deficit
                        }
                    })
                    .or_insert(deficit);
            }
        }
        prev_proc_idx = proc_idx;
    }
    current_needs
}

pub fn gen_initial_pop(spec: &Spec, process_nbr: usize) {
    let mut pop: Population = Default::default();
    for _ in 0..MAX_POPULATION {
        let prio_process_q = gen_rand_prio_process_q(process_nbr);
        println!("prio_process_q: {:?}", prio_process_q);
        let cand: Genome = Genome::new(prio_process_q, vec![], 0, spec.clone());
        pop.candidates.push(cand);
        // let cand: Genome = Genome::new(priority_process_q, max_job_cnt_perc_to_total_jobs);
    }
    calc_fitness(&mut pop);
}
