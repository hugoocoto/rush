use core::time;
use std::{
    collections::{HashMap, HashSet},
    env::{self, current_dir, set_current_dir},
    fs::{self},
    io::{Read, Write, stdin, stdout},
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
        let status = Command::new(e).args(arguments).status();
        if status.is_err() {
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
    if s.starts_with("-") {
        String::new()
    } else {
        suggest_path(s)
    }
}

fn suggest_path(s: &str) -> String {
    let p = Path::new(s);
    let dir = if p.is_dir() {
        p
    } else {
        p.parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new("."))
    };
    let prefix = if p.is_dir() {
        ""
    } else {
        p.file_name()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("")
    };

    if let Ok(read_dir) = fs::read_dir(dir) {
        let matches: Vec<String> = read_dir
            .filter_map(|entry| {
                let name = entry.ok()?.file_name();
                let name_str = name.to_str()?;
                if name_str.starts_with(prefix) {
                    Some(name_str.to_string())
                } else {
                    None
                }
            })
            .collect();

        match matches.len() {
            0 => String::new(),
            1 => {
                let completion = &matches[0][prefix.len()..];
                if dir.join(matches[0].clone()).is_dir() {
                    format!("{}/", completion)
                } else {
                    format!("{} ", completion)
                }
            }
            _ => {
                for match_name in &matches {
                    print!("{}\r\n", match_name);
                }
                String::from(&lcp(matches.iter().collect())[prefix.len()..])
            }
        }
    } else {
        print!("Error reading dir {}\r\n", dir.display());
        String::new()
    }
}

fn suggest_command_or_path(s: &str, com: &HashSet<String>) -> String {
    if s.starts_with("./") {
        suggest_path(s)
    } else {
        suggest_command(s, com)
    }
}

fn lcp(mut strs: Vec<&String>) -> String {
    if strs.is_empty() {
        return String::new();
    }
    strs.sort();

    let first = &strs[0];
    let last = &strs[strs.len() - 1];

    first
        .chars()
        .zip(last.chars())
        .take_while(|(a, b)| a == b)
        .map(|(a, _)| a)
        .collect()
}

fn suggest_command(s: &str, com: &HashSet<String>) -> String {
    let matches: Vec<&String> = com.iter().filter(|c| c.starts_with(s)).collect();
    match matches.len() {
        0 => String::new(),
        1 => {
            let mut suffix = String::from(&matches[0][s.len()..]);
            suffix.push(' ');
            suffix
        }
        _ => {
            matches
                .clone()
                .into_iter()
                .for_each(|x| print!("{}\r\n", x));
            String::from(&lcp(matches)[s.len()..])
        }
    }
}

fn suggest(s: String, c: &HashSet<String>) -> String {
    let args = parse(s.clone());
    match args.len() {
        0 => String::new(),
        1 => suggest_command_or_path(&args[0][..], c), // todo: suggest path if starting with ./
        _ => suggest_argument_or_path(args.get(args.len() - 1).unwrap()),
    }
}

fn load_commands_from_path(p: PathBuf, shell_commands: &mut HashSet<String>) {
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

fn load_commands(shell_commands: &mut HashSet<String>) {
    match env::var_os("PATH") {
        Some(paths) => {
            for path in env::split_paths(&paths) {
                load_commands_from_path(path, shell_commands);
            }
        }
        None => println!("PATH is not defined in the environment."),
    }
}

fn hello(command: Vec<String>) {
    assert!(command[0] == "hello");
    println!("Hello, World!");
}

fn cd(command: Vec<String>) {
    assert!(command[0] == "cd");
    set_current_dir(Path::join(
        Path::new(&current_dir().unwrap()),
        Path::new(&command[1]),
    ))
    .unwrap_or_else(|e| print!("{}: {e}\r\n", command.join(" ")));
}

pub fn main() {
    let mut builtin_table = HashMap::<String, Box<Builtin>>::new();
    let mut shell_commands = HashSet::new();

    builtin_table.insert(String::from("hello"), Box::new(hello));
    builtin_table.insert(String::from("cd"), Box::new(cd));
    load_commands(&mut shell_commands);

    'mainloop: loop {
        let mut input = String::new();
        print!("{PROMPT}");
        stdout().flush().unwrap();
        enable_raw_mode();
        loop {
            let ch: &mut [u8] = &mut [0];
            if let Ok(n) = stdin().read(ch) {
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
