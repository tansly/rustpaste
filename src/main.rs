use rustpaste::Config;

use std::process;

fn main() {
    let config = Config::new().unwrap();

    if let Err(e) = rustpaste::run(config) {
        println!("Application error: {}", e);
        process::exit(1);
    }
}
