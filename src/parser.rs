use std::fmt::Error;

use crate::Optimize;
use crate::Process;
use crate::Spec;
use crate::Stock;

pub fn parse_spec(input: &str) -> Result<Spec, String> {
    let mut processes: Vec<Process> = vec![];
    let mut stocks: Vec<Stock> = vec![];
    let mut optimize = None;

    for (index, line) in input.lines().enumerate() {
        if line.starts_with('#') {
            continue;
        } else if line.starts_with("optimize") {
            let opt = parse_optimize(line)?;
            if optimize.replace(opt).is_some() {
                return Err("Multiples optimize lines".to_string());
            }
        } else if !line.contains('(') {
            let stock = parse_stock(line)?;
            stocks.push(stock);
        } else {
            let process = parse_processes(line)?;
            processes.push(process);
        }
    }

    if optimize.is_none() {
        return Err("Missing optimization".to_string());
    }

    Ok(Spec::new(processes, stocks, optimize.unwrap()))
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
    let (name, rest) = input
        .split_once(':')
        .ok_or_else(|| "Invalid line".to_string())?;

    if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Err("Name of stock must only contain alphanumeric characters ".to_string());
    }

    let (rest, delay_str) = rest
        .rsplit_once(':')
        .ok_or_else(|| "Missing delay".to_string())?;

    let delay: i64 = match delay_str.parse::<i64>() {
        Ok(d) => d,
        Err(e) => return Err(format!("invalid delay: {}", e)),
    };

    let opening_brackets_cnt = rest.chars().filter(|&c| c == '(').count();

    match opening_brackets_cnt {
        0 | 3.. => return Err("Invalid line".to_string()),
        2 => {
            let (mut needs_str, mut results_str) = rest.split_once(')').ok_or_else(|| {
                "There must be at least a need or a result for a process".to_string()
            })?;

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

            return Ok(Process::new(name, needs, results, delay));
        }
        1 => {
            if rest.starts_with("(") {
                let needs_str = &rest[1..rest.len() - 2];
                let mut needs: Vec<Stock> = vec![];
                let results: Vec<Stock> = vec![];

                for pair in needs_str.split(';') {
                    let stock = parse_stock(pair)?;
                    needs.push(stock);
                }

                return Ok(Process::new(name, needs, results, delay));
            } else if rest.starts_with(":") {
                let results_str = &rest[2..rest.len() - 1];
                let needs: Vec<Stock> = vec![];
                let mut results: Vec<Stock> = vec![];

                for pair in results_str.split(';') {
                    let stock = parse_stock(pair)?;
                    results.push(stock);
                }

                return Ok(Process::new(name, needs, results, delay));
            } else {
                return Err("Invalid line".to_string());
            }
        }
    }
}

fn parse_optimize(input: &str) -> Result<Optimize, String> {
    let (_, mut optimize_str) = input
        .split_once(':')
        .ok_or_else(|| "Badly formatted optimize line".to_string())?;

    if !optimize_str.starts_with('(') || !optimize_str.ends_with(')') {
        return Err("Badly formatted optimize line".to_string());
    }

    optimize_str = &optimize_str[1..optimize_str.len() - 1];

    if optimize_str.chars().filter(|&c| c == ';').count() > 1 {
        return Err("Badly formatted optimize line".to_string());
    }

    println!("opt: [{}]", optimize_str);

    let optimize = match optimize_str.split_once(';') {
        None => Optimize::Quantity(optimize_str.to_string()),
        Some((left, right)) => {
            if left != "time" {
                return Err("Badly formatted optimize line".to_string());
            }
            Optimize::Time(right.to_string())
        }
    };

    //todo, has to be an existing name
    // if

    return Err("Invalid or missing optimize".to_string());
}
