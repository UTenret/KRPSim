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
            let process = parse_processes(line)?;
            processes.push(process);
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

    if !name.chars().all(|c| c.is_alphabetic() || c == '_') {
        return Err("Name of stock must only contain alphabetic characters ".to_string());
    }

    let qty: i64 = match qty_str.parse::<i64>() {
        Ok(n) => n,
        Err(e) => return Err(format!("invalid qty: {}", e)),
    };

    Ok(Stock::new(name, qty))
}

fn parse_processes(input: &str) -> Result<Process, String> {
    let mut process = Process::default();
    let (name, rest) = input
        .split_once(':')
        .ok_or_else(|| "Invalid line".to_string())?;

    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Name of stock must only contain alphanumeric characters ".to_string());
    }

    let (rest, delay) = rest
        .rsplit_once(':')
        .ok_or_else(|| "Missing delay".to_string())?;

    if !delay.chars().all(|c| c.is_numeric()) {
        return Err("Delay has to be a number ".to_string());
    }

    let opening_brackets_cnt = rest.chars().filter(|&c| c == '(').count();

    if opening_brackets_cnt > 2 {
        return Err("Syntax error, too many '(' in line".to_string());
    }

    if opening_brackets_cnt == 2 {
        let (mut needs_str, mut results_str) = rest
            .split_once(')')
            .ok_or_else(|| "There must be at least a need or a result for a process".to_string())?;

        if !needs_str.starts_with('(') {
            return Err("Syntax error, missing opening bracket".to_string());
        }

        needs_str = &needs_str[1..needs_str.len()];

        if !results_str.starts_with(":(") {
            return Err("Syntax error, missing opening bracket or colon".to_string());
        }

        if !results_str.ends_with(")") {
            return Err("Syntax error, missing closing bracket".to_string());
        }

        results_str = &results_str[2..results_str.len() - 1];

        let mut needs: Vec<Stock> = vec![];

        for pair in needs_str.split(';') {
            let stock = parse_stock(pair)?;
            needs.push(stock);
        }

        let mut results: Vec<Stock> = vec![];

        for pair in results_str.split(';') {
            let stock = parse_stock(pair)?;
            results.push(stock);
        }

        // println!("rest : [{}]", rest);
        // println!("needs_str : [{}]", needs_str);
        // println!("results_str : [{}]", results_str);
        println!("needs : [{:?}]", needs);
        println!("results : [{:?}]", results);
    }

    return Err("We are returning an error bahahah\n".to_string());
}

fn parse_optimize(input: &str) {}
