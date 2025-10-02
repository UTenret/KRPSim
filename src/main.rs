use std::env;
use std::fs;

struct Stock {
    name: String,
    quantity: i32,
}

impl Stock {
    pub fn new(name: &str, quantity: i32) -> Self {
        Self {
            name: name.to_string(),
            quantity,
        }
    }
}

struct Process {
    needs: Vec<Stock>,
    constructs: Vec<Stock>,
}

impl Process {
    fn new(needs: Vec<Stock>, constructs: Vec<Stock>) -> Self {
        Self { needs, constructs }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = &args[1];

    let planche = Stock::new("planche", 5);
    let casquette = Stock::new("casquette", 1);

    let become_a_baseball_player = Process::new(vec![planche, casquette], vec![]);

    let contents = fs::read_to_string(file_path);

    println!("File contents : {}", contents.unwrap());

    let mut cycle: i32 = 0;

    while (cycle < 2000) {
        cycle += 1;
    }
}
