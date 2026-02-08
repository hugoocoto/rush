use std::io::{Write, stdin, stdout};

const PROMPT: &str = ">> ";

fn run(input: &str) {
    let command = input.trim().split(" ").filter(|x| x.len() > 0);
    for p in command {
        print!("({p})");
    }
    println!("");
}

pub fn main() {
    println!("Hello, World!");

    loop {
        let mut input = String::new();
        print!("{PROMPT}");
        let _ = stdout().flush();
        let _ = stdin().read_line(&mut input);
        run(&input);
    }
}
