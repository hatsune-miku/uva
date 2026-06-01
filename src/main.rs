mod cli;
mod detect;
mod plan;
mod reqs;
mod runner;

use std::path::PathBuf;
use std::process::exit;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let plan = cli::dispatch(&args, &cwd);
    exit(runner::execute(plan));
}
