use core::time;
use std::{
    collections::{HashMap, HashSet},
    env,
    env::{current_dir, set_current_dir},
    fs,
    fs::File,
    io::{Read, Write, stderr, stdout},
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    process::Command,
    thread::sleep,
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

    if input.ends_with(" ") {
        arr.push(String::from(""));
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

fn enable_raw_mode() {
    Command::new("stty")
        .arg("raw")
        .arg("-echo")
        .status()
        .unwrap();
}

fn disable_raw_mode() {
    Command::new("stty").arg("sane").status().unwrap();
}

fn suggest_argument_or_path(s: &str) -> String {
    if let Ok(read_dir) = fs::read_dir(Path::new(".")) {
        let matches: Vec<String> = read_dir
            .filter_map(|entry| entry.ok().and_then(|e| e.file_name().into_string().ok()))
            .filter(|name| name.starts_with(s))
            .collect();

        match matches.len() {
            0 => String::new(),
            1 => {
                let mut completion = String::from(&matches[0][s.len()..]);
                completion.push(' ');
                completion
            }
            _ => {
                for match_name in matches {
                    print!("{}\r\n", match_name);
                }
                String::new()
            }
        }
    } else {
        String::new()
    }
}

fn suggest_command_name(s: &str, com: &HashSet<String>) -> String {
    let matches: Vec<&String> = com.iter().filter(|c| c.starts_with(s)).collect();

    match matches.len() {
        0 => String::new(),
        1 => {
            let full_match = matches[0];
            let mut suffix = String::from(&full_match[s.len()..]);
            suffix.push(' ');
            suffix
        }
        _ => {
            for match_name in matches {
                print!("{}\r\n", match_name);
            }
            String::new()
        }
    }
}

fn suggest(s: String, c: &HashSet<String>) -> String {
    let args = parse(s.clone());
    match args.len() {
        0 => String::new(),
        1 => suggest_command_name(&args[0][..], c), // todo: suggest path if starting with ./
        _ => suggest_argument_or_path(args.get(args.len() - 1).unwrap()),
    }
}

fn append_commands_from_path(p: PathBuf, shell_commands: &mut HashSet<String>) {
    if let Ok(dir) = p.read_dir() {
        for entry in dir {
            if let Ok(entry) = entry {
                if let Ok(metadata) = entry.metadata() {
                    let permissions = metadata.permissions();
                    if metadata.is_file() && permissions.mode() & 0o111 != 0 {
                        shell_commands.insert(entry.file_name().into_string().unwrap());
                    }
                }
            }
        }
    }
}

fn preload_commands(shell_commands: &mut HashSet<String>) {
    match env::var_os("PATH") {
        Some(paths) => {
            for path in env::split_paths(&paths) {
                append_commands_from_path(path, shell_commands);
            }
        }
        None => println!("PATH is not defined in the environment."),
    }
}

pub fn main() {
    let mut builtin_table = HashMap::<String, Box<Builtin>>::new();
    let mut shell_commands = HashSet::new();

    builtin_table.insert(String::from("hello"), Box::new(hello));
    builtin_table.insert(String::from("cd"), Box::new(cd));
    preload_commands(&mut shell_commands);

    let mut rawin = File::open("/dev/stdin").unwrap();
    'mainloop: loop {
        let mut input = String::new();
        print!("{PROMPT}");
        stdout().flush().unwrap();
        enable_raw_mode();
        loop {
            let ch: &mut [u8] = &mut [0];
            if let Ok(n) = rawin.read(ch) {
                if n == 0 {
                    sleep(time::Duration::from_millis(200));
                    continue;
                }
            }

            let ch = *ch.get(0).unwrap() as char;

            match ch {
                case if ch == 0x03 as char => {
                    // C-c
                    disable_raw_mode();
                    break 'mainloop;
                }

                case if ch == 0x0D as char => {
                    // CR (enter)
                    print!("\r\n");
                    stdout().flush().unwrap();
                    break;
                }

                case if ch == 0x08 as char || ch == 127 as char => {
                    // BS
                    if input.len() <= 0 {
                        continue;
                    }
                    print!("{}[D {}[D", 27 as char, 27 as char);
                    input.pop();
                    stdout().flush().unwrap();
                    continue;
                }

                case if ch == 0x09 as char => {
                    // TAB
                    print!("\n\r");
                    input.push_str(&suggest(input.clone(), &shell_commands)[..]);
                    print!("{PROMPT}{input}");
                    stdout().flush().unwrap();
                    continue;
                }

                case if ch == 0x00 as char => print!("NUL"),
                case if ch == 0x01 as char => print!("SOH"),
                case if ch == 0x02 as char => print!("STX"),
                case if ch == 0x04 as char => print!("EOT"),
                case if ch == 0x05 as char => print!("ENQ"),
                case if ch == 0x06 as char => print!("ACK"),
                case if ch == 0x07 as char => print!("BEL"),
                case if ch == 0x0A as char => print!("LF"),
                case if ch == 0x0B as char => print!("VT"),
                case if ch == 0x0C as char => print!("FF"),
                case if ch == 0x0E as char => print!("SO"),
                case if ch == 0x0F as char => print!("SI"),
                case if ch == 0x10 as char => print!("DLE"),
                case if ch == 0x11 as char => print!("DC1"),
                case if ch == 0x12 as char => print!("DC2"),
                case if ch == 0x13 as char => print!("DC3"),
                case if ch == 0x14 as char => print!("DC4"),
                case if ch == 0x15 as char => print!("NAK"),
                case if ch == 0x16 as char => print!("SYN"),
                case if ch == 0x17 as char => print!("ETB"),
                case if ch == 0x18 as char => print!("CAN"),
                case if ch == 0x19 as char => print!("EM"),
                case if ch == 0x1A as char => print!("SUB"),
                case if ch == 0x1B as char => print!("ESC"),
                case if ch == 0x1C as char => print!("FS"),
                case if ch == 0x1D as char => print!("GS"),
                case if ch == 0x1E as char => print!("RS"),
                case if ch == 0x1F as char => print!("US"),
                _ => {
                    print!("{ch}");
                    input.push(ch);
                }
            }
            stdout().flush().unwrap();
        }

        disable_raw_mode();

        if !input.is_empty() {
            run(input, &builtin_table);
        }
    }
}
