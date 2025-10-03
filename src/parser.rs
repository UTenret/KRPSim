use crate::Process;
use crate::Spec;
use crate::Stock;

pub fn parse_spec(input: &str) -> Result<Spec, String> {
    let mut processes: Vec<Process> = vec![];
    let mut stocks: Vec<Stock> = vec![];

    for (index, line) in input.lines().enumerate() {
        if line.starts_with('#') {
            continue;
        }

        if line.starts_with("optimize") {
            parse_optimize(line);
        }
    }

    Ok(Spec::new(processes, stocks))
}

fn parse_stock(input: &str) -> Result<Stock, String> {}

fn parse_processes(input: &str) -> Result<Process, String> {}

fn parse_optimize(input: &str) {}
