
use std::fs::File;
use std::io::Write;
use std::collections::HashMap;
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

        Self {file, headers}
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
