extern crate rand;

mod fdg;

use std::env;
use std::fs::File;
use std::path::Path;

fn main() {
    if let Some(loc) = env::args().nth(1) {
        let path = Path::new(&loc);
        let file = File::open(path).unwrap();
        let mut vm = fdg::VM::from_file(file);

        vm.run();
    } else {
        println!("Usage: fdg [file.fdg]")
    }
}
