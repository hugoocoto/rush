use std::{
    collections::HashMap,
    io::{Write, stderr, stdin, stdout},
    process::Command,
};

type Builtin = dyn Fn(Vec<&str>) -> ();
const PROMPT: &str = ">> ";

fn exec(input: Vec<&str>) {
    if input.len() == 0 {
        return;
    }
    let command = Command::new("sh")
        .arg("-c")
        .arg(input.join(" "))
        .output()
        .expect("Can't create command");
    let _ = stdout().write_all(&command.stdout);
    let _ = stderr().write_all(&command.stderr);
}

fn run(input: &str, builtins: &HashMap<&str, Box<Builtin>>) {
    let mut command: Vec<&str> = Vec::new();
    for elem in input.trim().split(" ") {
        if elem.len() > 0 {
            command.push(elem);
        }
    }
    if builtins.contains_key(command[0]) {
        builtins.get(command[0]).unwrap()(command);
    } else {
        exec(command);
    }
}

fn hello(_command: Vec<&str>) {
    println!("Hello, World!");
}

pub fn main() {
    let mut builtin_table: HashMap<&str, Box<Builtin>> = HashMap::new();
    builtin_table.insert("hello", Box::new(hello));

    loop {
        let mut input = String::new();
        print!("{PROMPT}");
        let _ = stdout().flush();
        let _ = stdin().read_line(&mut input);
        run(&input, &builtin_table);
    }
}
