use std::env;
use std::fs;

const CONFIG_DIR_PATH: &str = "config_files";

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = &args[1];

    let contents = fs::read_to_string(file_path);

    println!("File contents : {}", contents.unwrap());
}
