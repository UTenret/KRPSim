use std::{
    cmp::Reverse,
    collections::{BinaryHeap, HashMap},
    i64, vec,
};

use rand::{self, Rng, rng, seq::SliceRandom};

use crate::{Job, Optimize, Spec, Stock};

const ELITISM_CNT: i32 = 3;
const MAX_POPULATION: i32 = 200;
const PERCENT_CHANCE_TO_MUTATE: f64 = 3.0;
const MAX_CYCLES: i64 = 25000;

#[derive(Default)]
pub struct Population {
    candidates: Vec<Genome>,
}

#[derive(Clone)]
pub struct Genome {
    priority_process_q: Vec<usize>,
    current_needs: Vec<HashMap<String, i64>>,
    fitness: i64,
    spec: Spec, // max_job_cnt_perc_to_total_jobs: Vec<Vec<f64>>,
    pending_stock_divider: i32,
}

impl Genome {
    pub fn new(
        priority_process_q: Vec<usize>,
        current_needs: Vec<HashMap<String, i64>>,
        fitness: i64,
        spec: Spec,
        pending_stock_divider: i32,
        //max_job_cnt_perc_to_total_jobs: Vec<Vec<f64>>,
    ) -> Self {
        Self {
            priority_process_q,
            current_needs,
            fitness,
            spec, // max_job_cnt_perc_to_total_jobs,
            pending_stock_divider,
        }
    }
}

fn mutate() {}

fn crossover() {}

pub fn calc_fitness(pop: &mut Population) -> i64 {
    let mut genome_nbr = 0;
    for cand in &mut pop.candidates {
        let mut heap: BinaryHeap<Reverse<Job>> = BinaryHeap::new();
        let mut time = 1;

        let mut needs_maps: Vec<HashMap<String, i64>> = gen_current_needs(cand);
        let mut pending_stock: HashMap<String, i64> = Default::default();

        for cycles in 0..MAX_CYCLES {
            while let Some(top) = heap.peek() {
                if top.0.cycles == cycles {
                    let Reverse(job) = heap.pop().unwrap();
                    *cand.spec.stocks.entry(job.stock.name.clone()).or_insert(0) +=
                        job.stock.quantity;
                    if let Some(p) = pending_stock.get_mut(&job.stock.name.clone()) {
                        *p -= job.stock.quantity;
                        if *p <= 0 {
                            pending_stock.remove(&job.stock.name.clone());
                        }
                    }
                } else {
                    break;
                }
            }

            for (step_idx, &proc_idx) in cand.priority_process_q.iter().enumerate() {
                // eprintln!("{:#?}", needs_maps);
                // eprintln!("{:#?}", heap.len());
                // eprintln!("{:#?}", cand.spec.stocks);
                needs_maps = gen_current_needs(cand);
                // if cycles > 5270 {
                //     eprintln!("{:#?}", needs_maps);
                // }

                // 1) Can we run? (enough stocks for all needs)
                let can_run = cand.spec.processes[proc_idx].needs.iter().all(|need| {
                    let have = cand.spec.stocks.get(&need.name).copied().unwrap_or(0);
                    have >= need.quantity
                });
                if !can_run {
                    continue;
                }

                // 2) Should we run? (any result is currently needed at this step)
                // if the results of our proccesses
                // if any of those produces something we need
                // its a yes
                let should_run = cand.spec.processes[proc_idx].results.iter().any(|res| {
                    let have_now = cand.spec.stocks.get(&res.name).copied().unwrap_or(0)
                        + pending_stock.get(&res.name).copied().unwrap_or(0);

                    let need = needs_maps[step_idx].get(&res.name).copied().unwrap_or(0);

                    // if cycles > 5270 {
                    //     eprintln!(
                    //         "have now of {:?} is {} and need is : {}, cycle: {}",
                    //         &res.name, have_now, need, cycles
                    //     );
                    // }
                    // need > 0
                    // need - (pending_stock.get(&res.name).copied().unwrap_or(0) / 400) > 0
                    // eprintln!(
                    //     "pending {:?} for {}, need {}",
                    //     pending_stock.get(&res.name).unwrap_or(&0),
                    //     res.name,
                    //     need
                    // );
                    need - (pending_stock.get(&res.name).copied().unwrap_or(0)
                        / cand.pending_stock_divider as i64)
                        > 0
                    // need > have_now
                });
                // eprintln!("should_run : {}", should_run);
                if !should_run || cand.spec.processes[proc_idx].results.is_empty() {
                    continue;
                }

                // eprintln!("We are at cycle : {}", cycles);

                // eprintln!("Building {}", cand.spec.processes[proc_idx].name);
                // needs_maps = gen_current_needs(cand);
                // eprintln!("{:#?}", heap.len());
                // eprintln!("{:#?}", cand.spec.stocks);
                // eprintln!("{:#?}", needs_maps);

                for need in &cand.spec.processes[proc_idx].needs {
                    *cand.spec.stocks.entry(need.name.clone()).or_insert(0) -= need.quantity;
                }

                for stock in &cand.spec.processes[proc_idx].results {
                    heap.push(Reverse(Job {
                        cycles: cand.spec.processes[proc_idx].delay + cycles,
                        stock: stock.clone(),
                    }));
                    *pending_stock.entry(stock.name.clone()).or_insert(0) += stock.quantity;
                }
            }

            if heap.is_empty() {
                time = cycles;
                break;
            }
        }
        // eprintln!("{:#?}", cand.spec.stocks);
        let looking_for = match &cand.spec.optimize {
            Optimize::Time(s) | Optimize::Quantity(s) => s.as_str(),
        };
        for stock in &cand.spec.stocks {
            if stock.0 == looking_for {
                cand.fitness = *stock.1;
                // cand.fitness = *stock.1 / time;
                println!(
                    "Candidate {} => fitness before dividing {}, after : {} for stock: {} with time: {}",
                    genome_nbr, stock.1, cand.fitness, stock.0, time
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
    // vec![7, 6, 5, 4, 3, 2, 1, 0]
    // vec![13, 12, 14, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0]
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
            if let Optimize::Quantity(_) = &cand.spec.optimize {
                // eprintln!("{:?}", cand.spec.optimize);
                for final_prod in &cand.spec.processes[*&proc_idx].needs {
                    current_needs[c_needs_idx].insert(final_prod.name.clone(), i64::MAX);
                }
            }

            prev_proc_idx = proc_idx;
            continue;
        }

        // eprintln!("{}", c_needs_idx);
        current_needs[c_needs_idx] = current_needs[c_needs_idx - 1].clone();

        for need in &cand.spec.processes[prev_proc_idx].needs {
            let have = cand.spec.stocks.get(&need.name).copied().unwrap_or(0);
            let deficit = need.quantity - have;
            // eprintln!("deficit for {} is : {}", need.name, deficit);
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
            // eprintln!(
            //     "current_needs[c_needs_idx]
            //         .entry(need.name.clone()) {:?} ",
            //     current_needs[c_needs_idx].entry(need.name.clone())
            // );
        }
        prev_proc_idx = proc_idx;
    }
    current_needs
}

pub fn gen_pending_stock_divider() -> i32 {
    rand::rng().random_range(1..500)
}

pub fn gen_initial_pop(spec: &Spec, process_nbr: usize) -> Population {
    let mut pop: Population = Default::default();
    for _ in 0..MAX_POPULATION {
        let prio_process_q = gen_rand_prio_process_q(process_nbr);
        let divider = gen_pending_stock_divider();
        println!("prio_process_q: {:?}", prio_process_q);
        let cand: Genome = Genome::new(prio_process_q, vec![], 0, spec.clone(), divider);
        pop.candidates.push(cand);
    }
    eprintln!("{:.2}", rng().random::<f64>());
    pop
}

pub fn run_ga(mut pop: Population) -> Genome {
    calc_fitness(&mut pop);

    let mut best = pop
        .candidates
        .iter()
        .max_by_key(|c| c.fitness)
        // .unwrap()
        .clone();

    best.unwrap().clone()
}
