use std::{
    collections::HashMap,
    env::{current_dir, set_current_dir},
    io::{Write, stderr, stdin, stdout},
    path::Path,
    process::Command,
};

type Builtin = dyn Fn(Vec<String>) -> ();
const PROMPT: &str = ">> ";

fn exec(input: Vec<String>) {
    if let Some(e) = input.get(0) {
        let arguments = &input[1..];
        if let Ok(command) = Command::new(e).args(arguments).output() {
            stdout()
                .write_all(&command.stdout)
                .expect("Can't write to stdout");
            stderr()
                .write_all(&command.stderr)
                .expect("Can't write to stderr");
        } else {
            eprintln!("Command `{}` not found", input.get(0).unwrap());
        }
    }
}

fn parse(input: String) -> Vec<String> {
    let mut arr = Vec::<String>::new();
    let mut start = 0;
    let mut esq = false;
    let mut on_str = false;
    for i in 0..input.len() {
        if let Some(c) = input.chars().nth(i) {
            if esq {
                esq = false;
            } else if on_str {
                if c == '"' {
                    on_str = false;
                }
            } else if c == '"' {
                on_str = true;
            } else if c == '\\' {
                esq = true;
            } else if c.is_whitespace() {
                arr.push(String::from(input.get(start..i).unwrap()));
                start = i + 1;
            }
        } else {
            break;
        }
    }
    if start < input.len() {
        arr.push(String::from(input.get(start..).unwrap()));
    }

    return arr;
}

fn run(input: String, builtins: &HashMap<String, Box<Builtin>>) {
    let mut command: Vec<String> = Vec::new();
    for elem in parse(String::from(input.trim())) {
        if elem.len() > 0 {
            command.push(elem);
        }
    }
    if builtins.contains_key(&command[0]) {
        builtins.get(&command[0]).unwrap()(command);
    } else {
        exec(command);
    }
}

fn hello(command: Vec<String>) {
    assert!(command[0] == "hello");
    println!("Hello, World!");
}

fn cd(command: Vec<String>) {
    assert!(command[0] == "cd");
    let p = current_dir();
    set_current_dir(Path::join(Path::new(&p.unwrap()), Path::new(&command[1]))).expect("");
}

pub fn main() {
    let mut builtin_table: HashMap<String, Box<Builtin>> = HashMap::new();
    builtin_table.insert(String::from("hello"), Box::new(hello));
    builtin_table.insert(String::from("cd"), Box::new(cd));

    loop {
        let mut input = String::new();
        print!("{PROMPT}");
        stdout().flush().unwrap();
        stdin().read_line(&mut input).unwrap();
        run(input, &builtin_table);
    }
}
