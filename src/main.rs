// Capsules
//
// A podman wrapper/helper that manages "capsules", task-centric
// containers designed to isolate working environments and keep
// the host OS in good hygene.
//
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs, str};

use serde::Deserialize;

fn list_capsules() {
    let output = Command::new("podman")
        .arg("container")
        .arg("list")
        .arg("-a")
        .arg("--format")
        .arg("{{printf \"% -30s %-60s %-40s\" .Names .Image .Status}}")
        .output()
        .expect("Failed to list containers");

    let output_str = String::from_utf8_lossy(&output.stdout);

    for line in output_str.lines() {
        if line.contains("capsule") {
            println!("{}", line);
        }
    }
}

fn capsule_console_as_root(container_id: &str, command: Option<String>) {
    let _status = Command::new("podman")
        .arg("exec")
        .arg("-it")
        .arg("--user=root")
        .arg(format!("capsule-{}", container_id))
        .arg(command.unwrap_or("sh".to_string()))
        .spawn()
        .expect("Failed to execute command")
        .wait()
        .expect("Failed to wait on command");
}

fn execute_in_capsule(container_id: &str, command: &str) {
    Command::new("podman")
        .arg("start")
        .arg(format!("capsule-{}", container_id))
        .output()
        .expect("Failed to start container");

    let username_str = std::env::var("USER").unwrap_or_else(|_| {
        panic!("$USER is not set or cannot be used");
    });

    let _status = Command::new("podman")
        .arg("exec")
        .arg("-it")
        .arg("--user")
        .arg(username_str.trim())
        .arg(format!("capsule-{}", container_id))
        .arg(command)
        .spawn()
        .expect("Failed to execute command")
        .wait()
        .expect("Failed to wait on command");
}

fn spin_a_new_capsule(
    cfg: &Config,
    image: &str,
    container_id: &str,
    additional_volumes: Vec<String>,
    init: bool,
) {
    let source_path = format!(
        "{}/{}",
        get_user_config_folder().to_str().unwrap(),
        container_id
    );
    if init {
        if !std::path::Path::new(&source_path).exists() {
            panic!("Source path does not exist: {}", source_path);
        }
    }

    let capsule_username = std::env::var("USER").unwrap_or_else(|_| {
        panic!("$USER is not set or cannot be used");
    });

    let volumes_root = cfg.volumes_root_path();
    let home_path = volumes_root
        .join(container_id)
        .join("home")
        .join(&capsule_username);
    let bootstrap_path = volumes_root.join(container_id).join(".bootstrap");

    fs::create_dir_all(&home_path).expect("Failed to create home directory");

    let source_path = format!(
        "{}/{}",
        get_user_config_folder().to_str().unwrap(),
        container_id
    );
    let destination_path = bootstrap_path.to_str().unwrap().to_string();

    copy_directory(&source_path, &destination_path).expect("Failed to copy bootstrap script");

    let volumes_root_str = volumes_root
        .to_str()
        .expect("Volumes root path is not valid UTF-8");
    let volumes_path = &format!("{volumes_root_str}/{container_id}");
    let container_volume = format!("{volumes_path}:/files:rw");

    let mut command = Command::new("podman");

    let capsule_home_dir = cfg.capsule_home_dir();

    command
        .arg("run")
        .arg("-d")
        .arg("--gpus")
        .arg("all")
        .arg("-h")
        .arg(container_id)
        .arg("-e")
        .arg("DISPLAY=:0")
        .arg("--net=host")
        .arg("--userns=keep-id")
        .arg("--user=root")
        .arg("--pids-limit=-1")
        .arg("-v")
        .arg("/dev/snd:/dev/snd:rw")
        .arg("-v")
        .arg("/dev/shm:/dev/shm:rw")
        .arg("-v")
        .arg("/run/user/1000/pulse:/run/user/host/pulse:rw")
        .arg("-v")
        .arg(format!(
            "/files/projects/dotfiles/config:{capsule_home_dir}/{}/.config:rw",
            capsule_username
        ))
        .arg("-v")
        .arg(format!(
            "/files/projects/dotfiles/fonts:{capsule_home_dir}/{}/.fonts",
            capsule_username
        ))
        .arg("-v")
        .arg(container_volume);

    for additional_volume in additional_volumes {
        command.arg("-v").arg(additional_volume);
    }

    let output = command
        .arg("-e")
        .arg("PULSE_SERVER=unix:/run/user/host/pulse/native")
        .arg("-e")
        .arg(format!("BOOTSTRAP={}.sh", container_id))
        .arg("-e")
        .arg(format!("CAPSULE_HOMEDIR={capsule_home_dir}"))
        .arg("-e")
        .arg(format!("CAPSULE_USERNAME={}", capsule_username))
        .arg("--name")
        .arg(format!("capsule-{}", container_id))
        .arg(image)
        .arg("sleep")
        .arg("infinity")
        .output()
        .expect("Failed to execute podman command");

    println!(
        "Container spun up: {} {}",
        String::from_utf8_lossy(&output.stdout),
        output.status
    );

    if init {
        let _status = Command::new("podman")
            .arg("exec")
            .arg("--user=root")
            .arg(format!("capsule-{}", container_id))
            .arg("bash")
            .arg("/files/.bootstrap/init.sh")
            .spawn()
            .expect("Failed to execute command")
            .wait()
            .expect("Failed to wait on command");
    }
}

fn delete_capsule(container_id: &str) {
    let output = Command::new("podman")
        .arg("rm")
        .arg("-f")
        .arg(format!("capsule-{}", container_id))
        .output()
        .expect("Failed to execute command");
    println!(
        "Container deleted: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn start_capsule(container_id: &str) {
    let output = Command::new("podman")
        .arg("start")
        .arg(format!("capsule-{}", container_id))
        .output()
        .expect("Failed to execute command");
    println!(
        "Container started: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn stop_capsule(container_id: &str) {
    let output = Command::new("podman")
        .arg("stop")
        .arg(format!("capsule-{}", container_id))
        .output()
        .expect("Failed to execute command");
    println!(
        "Container stopped: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

fn main() {
    let cfg = Config::load();
    let matches = clap::Command::new("Capsules")
        .about("Secure-by-default containers for operating-system hygene")
        .version("1.0")
        .author("Ithai Levi")
        .subcommand(clap::Command::new("list").about("List all capsules"))
        .subcommand(
            clap::Command::new("console")
                .about("Start a console root session")
                .arg(clap::Arg::new("container_id").required(true))
                .arg(clap::Arg::new("command").required(false)),
        )
        .subcommand(
            clap::Command::new("exec")
                .about("Executes a command in a running container")
                .arg(clap::Arg::new("container_id").required(true))
                .arg(clap::Arg::new("command").required(true)),
        )
        .subcommand(
            clap::Command::new("spin")
                .about("Spins up a new container")
                .arg(clap::Arg::new("image").required(true))
                .arg(clap::Arg::new("container_id").required(true))
                .arg(
                    clap::Arg::new("volume")
                        .long("volume")
                        .short('v')
                        .num_args(1..)
                        .value_name("host_path:container_path")
                        .help("Bind mount a volume (can be used multiple times)"),
                )
                .arg(
                    clap::Arg::new("no-init")
                        .short('f')
                        .long("no-init")
                        .required(false)
                        .action(clap::ArgAction::SetFalse),
                ),
        )
        .subcommand(
            clap::Command::new("start")
                .about("Starts a container")
                .arg(clap::Arg::new("container_id").required(true)),
        )
        .subcommand(
            clap::Command::new("stop")
                .about("Stops a container")
                .arg(clap::Arg::new("container_id").required(true)),
        )
        .subcommand(
            clap::Command::new("delete")
                .about("Deletes a container")
                .arg(clap::Arg::new("container_id").required(true)),
        )
        .get_matches();

    if let Some(_) = matches.subcommand_matches("list") {
        list_capsules();
    }

    if let Some(matches) = matches.subcommand_matches("console") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        capsule_console_as_root(container_id, matches.get_one::<String>("command").cloned());
    }

    if let Some(matches) = matches.subcommand_matches("exec") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        let command = matches.get_one::<String>("command").unwrap();
        execute_in_capsule(container_id, command);
    }

    if let Some(matches) = matches.subcommand_matches("spin") {
        let image = matches.get_one::<String>("image").unwrap();
        let init = matches.get_flag("no-init");
        let container_id = matches.get_one::<String>("container_id").unwrap();
        let volumes = match matches.get_many::<String>("volume") {
            Some(volumes) => volumes.map(|v| v.to_owned()).collect(),
            None => Vec::new(),
        };
        spin_a_new_capsule(&cfg, image, container_id, volumes, init);
    }

    if let Some(matches) = matches.subcommand_matches("delete") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        delete_capsule(container_id);
    }

    if let Some(matches) = matches.subcommand_matches("stop") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        stop_capsule(container_id);
    }

    if let Some(matches) = matches.subcommand_matches("start") {
        let container_id = matches.get_one::<String>("container_id").unwrap();
        start_capsule(container_id);
    }
}

fn get_user_config_folder() -> PathBuf {
    let home_dir = env::home_dir().expect("Could not get home directory");
    home_dir.join(".config").join("capsules").join("bootstrap")
}

fn copy_directory(src: &str, dst: &str) -> std::io::Result<()> {
    let src_path = Path::new(src);
    let dst_path = Path::new(dst);

    if src_path.is_dir() {
        fs::create_dir_all(dst_path)?;
        for entry in fs::read_dir(src_path)? {
            let entry = entry?;
            let entry_path = entry.path();
            let new_path = dst_path.join(entry.file_name());

            if entry_path.is_dir() {
                copy_directory(entry_path.to_str().unwrap(), new_path.to_str().unwrap())?;
            } else {
                fs::copy(entry_path, new_path)?;
            }
        }
    }
    Ok(())
}

#[derive(Debug, Deserialize, Clone)]
struct Config {
    volumes_root: Option<String>,
    capsule_home_dir: Option<String>,
}

impl Config {
    fn load() -> Self {
        let home_dir = env::home_dir().expect("Could not get home directory");
        let config_dir = home_dir.join(".config").join("capsules");
        let config_file = config_dir.join("capsules.toml");

        if let Ok(contents) = fs::read_to_string(&config_file) {
            if let Ok(cfg) = toml::from_str::<Config>(&contents) {
                return cfg;
            }
        }

        Config {
            volumes_root: None,
            capsule_home_dir: None,
        }
    }

    fn volumes_root_path(&self) -> PathBuf {
        let home_dir = env::home_dir().expect("Could not get home directory");

        if let Some(ref root) = self.volumes_root {
            let path = PathBuf::from(root);
            if path.is_absolute() {
                path
            } else {
                home_dir.join(path)
            }
        } else {
            home_dir.join(".local").join("capsules").join("volumes")
        }
    }

    fn capsule_home_dir(&self) -> &str {
        self.capsule_home_dir.as_deref().unwrap_or("/home")
    }
}
