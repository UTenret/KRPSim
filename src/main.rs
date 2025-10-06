use std::env;
use std::fs;
use std::os::unix::process;
use std::process::exit;

mod parser;

#[derive(Debug)]
pub struct Stock {
    name: String,
    quantity: i64,
}

impl Stock {
    pub fn new(name: &str, quantity: i64) -> Self {
        Self {
            name: name.to_string(),
            quantity,
        }
    }
}

#[derive(Default)]
pub struct Process {
    name: String,
    needs: Vec<Stock>,
    results: Vec<Stock>,
    delay: i64,
}

impl Process {
    fn new(name: &str, needs: Vec<Stock>, results: Vec<Stock>, delay: i64) -> Self {
        Self {
            name: name.to_string(),
            needs,
            results,
            delay,
        }
    }
}

pub struct Spec {
    processes: Vec<Process>,
    stocks: Vec<Stock>,
    // optimize: Option<Time, String>, // need optimize there too
}

impl Spec {
    fn new(processes: Vec<Process>, stocks: Vec<Stock>) -> Self {
        Self { processes, stocks }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() == 1 {
        println!("You need to specify a file");
        std::process::exit(0);
    }

    let file_path = &args[1];

    let contents = fs::read_to_string(file_path).unwrap_or_else(|e| {
        eprintln!("Failed to read the contents of the file : {}", e);
        exit(1);
    });

    let res = parser::parse_spec(&contents).unwrap_or_else(|e| {
        eprintln!("Error while parsing the contents of the file : {}", e);
        exit(1);
    });

    // println!("File contents : {}", contents.unwrap());

    // let mut cycle: i32 = 0;

    // while (cycle < 2000) {
    //     cycle += 1;
    // }
}
