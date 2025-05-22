use std::path::Path;
use std::process::Command;
use std::{fs, os::unix::fs::PermissionsExt};

use fs_extra::{copy_items, dir};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const SANDBOX_FOLDER: &str = "sandbox";

#[derive(Serialize, Deserialize)]
pub struct FormData<'a> {
    pub commands: Vec<CMD>,
    pub image: &'a str,
    pub submit_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct ResponseData {
    pub sandbox_results: Vec<SandboxResult>,
    pub submit_id: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Config {
    pub time_limit: u64,
    pub time_reserved: u64,
    pub memory_limit: u64,
    pub memory_reserved: u64,
    pub large_stack: bool,
    pub output_limit: u64,
    pub process_limit: u64,
}

// impl Config {
//     fn new(image: String, stdin: String) -> Self {
//         Config {
//             time_limit: 1,
//             time_reserved: 1,
//             memory_limit: 256000,
//             memory_reserved: 6144000,
//             large_stack: false,
//             output_limit: 0,
//             process_limit: 0,
//         }
//     }
// }

// Command to be executed
#[derive(Serialize, Deserialize, Clone)]
pub struct CMD {
    pub command: String,
    pub args: Vec<String>,
    pub input: String,
    pub config: Config,
}

// Enum representing the exit state of the sandboxed process
#[derive(Serialize, Deserialize, Debug)]
enum ExitState {
    Success,
    RuntimeError,
    TimeLimitExceeded,
    MemoryLimitExceeded,
    OtherError,
}

// Structure to hold the result of the sandbox execution
#[derive(Serialize, Deserialize, Debug)]
pub struct SandboxResult {
    state: ExitState,
    stdout: String,
    stderr: String,
    time: u64,   // Execution time in seconds
    memory: u64, // Memory usage in KB
}

pub fn sandbox_service(
    commands: Vec<CMD>,
    image: &str,
) -> Result<Vec<SandboxResult>, Box<dyn std::error::Error>> {
    if !Path::new(SANDBOX_FOLDER).exists() {
        panic!("No sandbox found");
    }
    let tmp_folder = Uuid::new_v4().to_string();
    if !Path::new(&tmp_folder).exists() {
        fs::create_dir(&tmp_folder).unwrap();
    }
    let perm = fs::Permissions::from_mode(0o777);
    fs::set_permissions(&tmp_folder, perm.clone())?;
    copy_items(
        &[format!("{}/sandbox", SANDBOX_FOLDER)],
        &tmp_folder,
        &dir::CopyOptions::new(),
    )
    .unwrap();

    fs::write(
        format!("{}/commands.yaml", &tmp_folder),
        serde_yaml::to_string(&commands).unwrap(),
    )
    .unwrap();

    let mut command = Command::new("docker");
    command.arg("run").arg("--rm");
    command.arg("-v").arg(format!("./{}:/sandbox", tmp_folder));
    command.arg("-w").arg(format!("/{}", SANDBOX_FOLDER));
    command.arg(image).arg("./sandbox");
    let _ = command.output();
    let results = fs::read_to_string(format!("{}/results.yaml", tmp_folder)).unwrap();
    let _ = fs::remove_dir_all(tmp_folder);
    Ok(serde_yaml::from_str(&results).unwrap())
}

#[cfg(test)]
mod service_test {

    use super::*;

    #[test]
    fn gcc_version() {
        let commands = vec![CMD {
            command: "gcc".to_string(),
            args: vec!["--version".to_string()],
            input: "".to_string(),
            config: Config {
                time_limit: 1,
                time_reserved: 1,
                memory_limit: 256000,
                memory_reserved: 4096000,
                large_stack: false,
                output_limit: 0,
                process_limit: 0,
            },
        }];
        let results = sandbox_service(commands, "gcc:14.2");
        assert!(results.is_ok());
        assert_eq!(
            results.unwrap()[0].stdout,
            "gcc (GCC) 14.2.0\nCopyright (C) 2024 Free Software Foundation, Inc.\nThis is free software; see the source for copying conditions.  There is NO\nwarranty; not even for MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.\n\n"
        )
    }

    #[test]
    fn cpp_a_add_b() {
        let commands = vec![
            CMD {
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    r#"echo '#include <iostream>
using namespace std;
int main() {
    int a, b;
    cin >> a >> b;
    cout << a << " + " << b << " = " << a + b << endl;
}' > main.cpp"#
                        .to_string(),
                ],
                input: "".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 4096000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
            CMD {
                command: "g++".to_string(),
                args: vec!["main.cpp".to_string(), "-o".to_string(), "main".to_string()],
                input: "".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 4096000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
            CMD {
                command: "./main".to_string(),
                args: vec![],
                input: "1 2".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 4096000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
        ];
        let results = sandbox_service(commands, "gcc:14.2");
        assert!(results.is_ok());
        assert_eq!(results.unwrap()[2].stdout, "1 + 2 = 3\n")
    }

    #[test]
    fn java_version() {
        let commands = vec![CMD {
            command: "java".to_string(),
            args: vec!["--version".to_string()],
            input: "".to_string(),
            config: Config {
                time_limit: 1,
                time_reserved: 1,
                memory_limit: 256000,
                memory_reserved: 6144000,
                large_stack: false,
                output_limit: 0,
                process_limit: 0,
            },
        }];
        let results = sandbox_service(commands, "openjdk:21");
        assert!(results.is_ok());
        assert_eq!(
            results.unwrap()[0].stdout,
            "openjdk 21 2023-09-19\nOpenJDK Runtime Environment (build 21+35-2513)\nOpenJDK 64-Bit Server VM (build 21+35-2513, mixed mode, sharing)\n"
        )
    }

    #[test]
    fn java_a_add_b() {
        let commands = vec![
            CMD {
                command: "sh".to_string(),
                args: vec![
                    "-c".to_string(),
                    r#"echo 'import java.util.Scanner;
public class Main {
    public static void main(String[] args) {
        Scanner sc = new Scanner(System.in);
        int a = sc.nextInt();
        int b = sc.nextInt();
        System.out.println(a + " + " + b + " = " + (a + b));
    }
}' > Main.java"#
                        .to_string(),
                ],
                input: "".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 6144000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
            CMD {
                command: "javac".to_string(),
                args: vec!["Main.java".to_string()],
                input: "".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 6144000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
            CMD {
                command: "java".to_string(),
                args: vec!["Main.java".to_string()],
                input: "1 2".to_string(),
                config: Config {
                    time_limit: 1,
                    time_reserved: 1,
                    memory_limit: 256000,
                    memory_reserved: 6144000,
                    large_stack: false,
                    output_limit: 0,
                    process_limit: 0,
                },
            },
        ];
        let results = sandbox_service(commands, "openjdk:21");
        assert!(results.is_ok());
        assert_eq!(results.unwrap()[2].stdout, "1 + 2 = 3\n")
    }

    #[test]
    fn robust0() {
        let commands = vec![CMD {
            command: "reboot".to_string(),
            args: vec![],
            input: "".to_string(),
            config: Config {
                time_limit: 1,
                time_reserved: 1,
                memory_limit: 256000,
                memory_reserved: 6144000,
                large_stack: false,
                output_limit: 0,
                process_limit: 0,
            },
        }];
        let results = sandbox_service(commands, "gcc:14.2");
        assert!(results.is_ok());
        assert_eq!(
            format!("{:?}", results.unwrap()),
            r#"[SandboxResult { state: OtherError, stdout: "", stderr: "Error occurred", time: 0, memory: 0 }]"#
        );
    }

    #[test]
    fn robust1() {
        let commands = vec![CMD {
            command: "rm".to_string(),
            args: vec!["-rf".to_string(), "/*".to_string()],
            input: "".to_string(),
            config: Config {
                time_limit: 1,
                time_reserved: 1,
                memory_limit: 256000,
                memory_reserved: 6144000,
                large_stack: false,
                output_limit: 0,
                process_limit: 0,
            },
        }];
        let results = sandbox_service(commands, "gcc:14.2");
        assert!(results.is_ok());
    }
}
