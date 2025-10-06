use std::fmt::Error;

use crate::Process;
use crate::Spec;
use crate::Stock;

pub fn parse_spec(input: &str) -> Result<Spec, String> {
    let mut processes: Vec<Process> = vec![];
    let mut stocks: Vec<Stock> = vec![];

    for (index, line) in input.lines().enumerate() {
        if line.starts_with('#') {
            continue;
        } else if line.starts_with("optimize") {
            parse_optimize(line);
        } else if !line.contains('(') {
            let stock = parse_stock(line)?;
            stocks.push(stock);
        } else {
            // let process = parse_processes(line)?;
            // processes.push(process);
        }
    }

    Ok(Spec::new(processes, stocks))
}

fn parse_stock(input: &str) -> Result<Stock, String> {
    // todo add line number to error
    let (name, qty_str) = input
        .split_once(':')
        .ok_or_else(|| "Invalid line".to_string())?;

    if qty_str.contains(':') {
        return Err("Too many \':\' for stock line".to_string());
    }

    if !name.chars().all(char::is_alphabetic) {
        return Err("Name of stock must only contain alphabetic characters ".to_string());
    }

    let qty: i64 = match qty_str.parse::<i64>() {
        Ok(n) => n,
        Err(e) => return Err(format!("invalid qty: {}", e)),
    };

    // println!("{:?}, {:?}", words.clone().count(), words);
    // return Ok(Stock::new(words))

    return Err("We are returning an error ahahahah\n".to_string());
}

fn parse_processes(input: &str) -> Result<Process, String> {
    return Err("We are returning an error bahahah\n".to_string());
}

fn parse_optimize(input: &str) {}
