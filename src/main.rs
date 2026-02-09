use std::{
    collections::HashMap,
    io::{Write, stderr, stdin, stdout},
    process::Command,
};

type Builtin = dyn Fn(Vec<String>) -> ();
const PROMPT: &str = ">> ";

fn exec(input: Vec<String>) {
    if let Some(e) = input.get(0) {
        let arguments = &input[1..];
        for a in arguments {
            print!("({a})");
        }
        println!("");
        let command = Command::new(e)
            .args(arguments)
            .output()
            .expect("Can't create command");
        stdout()
            .write_all(&command.stdout)
            .expect("Can't write to stdout");
        stderr()
            .write_all(&command.stderr)
            .expect("Can't write to stderr");
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

fn hello(_command: Vec<String>) {
    assert!(_command[0] == "hello");
    println!("Hello, World!");
}

pub fn main() {
    let mut builtin_table: HashMap<String, Box<Builtin>> = HashMap::new();
    builtin_table.insert(String::from("hello"), Box::new(hello));

    loop {
        let mut input = String::new();
        print!("{PROMPT}");
        stdout().flush().unwrap();
        stdin().read_line(&mut input).unwrap();
        run(input, &builtin_table);
    }
}
