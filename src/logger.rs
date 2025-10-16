use std::collections::HashMap;
use std::fs::File;
use std::io::Write;

use crate::ga::{Genome, priority_from_keys};
use crate::{Optimize, Stock};
pub struct Logger {
    file: File,
    headers: Vec<String>,
}

impl Logger {
    pub fn new(stocks: &HashMap<String, i64>, filename: &str) -> Self {
        let mut file = File::create(filename).unwrap();
        let headers: Vec<String> = stocks.keys().cloned().collect();
        let header_line = format!("time,{}", headers.join(","));
        writeln!(&mut file, "{}", header_line).unwrap();

        Self { file, headers }
    }

    pub fn log_stocks(&mut self, time: i64, stocks: &HashMap<String, i64>) {
        let row: Vec<String> = self
            .headers
            .iter()
            .map(|k| stocks.get(k).unwrap_or(&0).to_string())
            .collect();
        writeln!(self.file, "{},{}", time, row.join(",")).unwrap();
    }
}

pub fn print_genome(genome: &Genome) {
    let order = priority_from_keys(&genome.keys);
    let spec = &genome.spec;

    // Header
    println!("====================== GENOME ======================");
    println!("fitness                 : {}", genome.fitness);
    println!("pending_stock_divider   : {}", genome.pending_stock_divider);
    println!("disabled_processes flag : {}", genome.disabled_processes);

    // Optimize target
    match &spec.optimize {
        Optimize::Quantity(name) => println!("optimize                 : Quantity({})", name),
        Optimize::Time(name) => println!("optimize                 : Time({})", name),
    }

    // Initial stocks
    // println!("\n--- Initial stocks ---------------------------------");
    // let mut init: Vec<_> = spec.init_stocks.iter().collect();
    // init.sort_by(|a, b| a.0.cmp(b.0));
    // if init.is_empty() {
    //     println!("(none)");
    // } else {
    //     for (name, qty) in init {
    //         println!("  {:<16} {}", name, qty);
    //     }
    // }

    // Disabled processes (keys == 1.0)
    let disabled: Vec<usize> = genome
        .keys
        .iter()
        .enumerate()
        .filter_map(|(i, &k)| if k == 1.0 { Some(i) } else { None })
        .collect();
    if !disabled.is_empty() {
        println!("\n--- Disabled processes ------------------------------");
        println!("  {:?}", disabled);
    }

    // Table header
    println!("\n--- Processes (priority order: low key = higher prio) -----");
    println!(
        "{:<4} {:<6} {:<12} {:<8} {:<8} {:<40} {:<40}",
        "rank", "pid", "key", "wait", "dur", "needs", "results"
    );

    for (rank, &pid) in order.iter().enumerate() {
        let p = &spec.processes[pid];
        let key = genome.keys[pid];
        let needs = fmt_stock_list(&p.needs);
        let results = fmt_stock_list(&p.results);

        println!(
            "{:<4} {:<6} {:<12.6}  {:<8} {:<40} {:<40}",
            rank, pid, key, p.duration, needs, results
        );
    }

    // Also show wait cycles in priority order as a single line (handy for debugging)

    println!("====================================================\n");
}

// Helper to format a Vec<Stock> like "3 iron, 1 copper"
fn fmt_stock_list(v: &[Stock]) -> String {
    if v.is_empty() {
        return "-".to_string();
    }
    v.iter()
        .map(|s| format!("{} {}", s.quantity, s.name))
        .collect::<Vec<_>>()
        .join(", ")
}
