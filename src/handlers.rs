use std::process::{Command, Stdio};
use std::{env, fs};

use anyhow::{Context, Result};

use crate::utils::{copy_directory, get_user_config_folder, CapsuleFile, Config};

pub fn list_capsules() -> Result<()> {
    let output = Command::new("podman")
        .arg("container")
        .arg("list")
        .arg("-a")
        .arg("--format")
        .arg("{{printf \"% -30s %-60s %-40s\" .Names .Image .Status}}")
        .output()
        .context("Failed to list containers")?;

    let output_str = String::from_utf8_lossy(&output.stdout);

    for line in output_str.lines() {
        if line.contains("capsule") {
            println!("{}", line);
        }
    }
    Ok(())
}

pub fn capsule_console_as_root(container_id: &str, command: Option<String>) -> Result<()> {
    Command::new("podman")
        .arg("exec")
        .arg("-it")
        .arg("--user=root")
        .arg(format!("capsule-{}", container_id))
        .arg(command.unwrap_or("sh".to_string()))
        .spawn()
        .context("Failed to spawn console command")?
        .wait()
        .context("Failed to wait on console command")?;
    Ok(())
}

pub fn execute_in_capsule(container_id: &str, command: &str, args: Vec<String>) -> Result<()> {
    Command::new("podman")
        .arg("start")
        .arg(format!("capsule-{}", container_id))
        .output()
        .context("Failed to start container")?;

    let username_str = env::var("USER").context("$USER is not set or cannot be used")?;

    let mut cmd = Command::new("podman");
    cmd.arg("exec")
        .arg("-it")
        .arg("--user")
        .arg(username_str.trim())
        .arg(format!("capsule-{}", container_id))
        .arg(command);

    for arg in args {
        cmd.arg(arg);
    }

    cmd.spawn()
        .context("Failed to execute command")?
        .wait()
        .context("Failed to wait on command")?;
    Ok(())
}

pub fn init_container_volume(_cfg: &Config, blueprint: &str) -> Result<()> {
    let source_path = get_user_config_folder().join(blueprint);
    if !source_path.exists() {
        anyhow::bail!("Source path does not exist: {}", source_path.display());
    }

    let volumes_path = env::current_dir().context("Failed to get current directory")?;
    let capsule_dir = volumes_path.join(".capsules");

    if capsule_dir.exists() {
        anyhow::bail!(".capsules already exists in the current directory");
    }

    copy_directory(
        source_path.to_str().unwrap(),
        &capsule_dir.to_str().unwrap(),
    )
    .context("Failed to copy bootstrap script")?;

    let mut tar = Command::new("tar")
        .args(["-czh", "."])
        .current_dir(&capsule_dir)
        .stdout(Stdio::piped())
        .spawn()
        .context("Failed to spawn tar")?;

    let mut podman = Command::new("podman")
        .args(["build", "-t", blueprint, "-"])
        .stdin(tar.stdout.take().unwrap())
        .spawn()
        .context("Failed to spawn podman build")?;

    let podman_status = podman.wait().context("Failed to wait on podman build")?;
    if !podman_status.success() {
        anyhow::bail!("podman build failed: {podman_status}");
    }

    Ok(())
}

pub fn spin_a_new_capsule(
    cfg: &Config,
    container_id: &str,
    additional_volumes: Vec<String>,
    init: bool,
) -> Result<()> {
    let exists = Command::new("podman")
        .args(["container", "exists", &format!("capsule-{}", container_id)])
        .output()
        .context("Failed to check container existence")?;

    if exists.status.success() {
        anyhow::bail!("Capsule '{}' already exists", container_id);
    }

    let capsule_username = env::var("USER").context("$USER is not set or cannot be used")?;

    let volumes_path = env::current_dir().context("Failed to get current directory")?;
    let home_path = volumes_path
        .join(cfg.capsule_home_dir())
        .join(&capsule_username);

    fs::create_dir_all(&home_path).context("Failed to create home directory")?;

    let capsule_file = volumes_path.join(".capsules").join("capsule.toml");

    let contents = fs::read_to_string(&capsule_file)
        .with_context(|| format!("Failed to read {:?}", capsule_file))?;
    let capsule_desc: CapsuleFile = toml::from_str(&contents).context("Failed to parse TOML")?;
    let blueprint = capsule_desc
        .blueprint
        .context("Missing blueprint in capsule.toml")?;

    let volumes_path_as_str = volumes_path.to_str().unwrap().to_string();
    let capsule_volume_dir = cfg.capsule_volume_dir();

    let container_volume = format!("{volumes_path_as_str}:{capsule_volume_dir}:rw");

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
        .arg(format!(
            "DISPLAY={}",
            env::var("DISPLAY").unwrap_or_else(|_| ":0".to_string())
        ))
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
        .arg(format!(
            "CAPSULE_HOMEDIR={capsule_volume_dir}/{capsule_home_dir}"
        ))
        .arg("-e")
        .arg(format!("CAPSULE_USERNAME={}", capsule_username))
        .arg("--name")
        .arg(format!("capsule-{}", container_id))
        .arg(blueprint)
        .arg("sleep")
        .arg("infinity")
        .output()
        .context("Failed to execute podman run command")?;

    println!(
        "Container spun up: {} {}",
        String::from_utf8_lossy(&output.stdout),
        output.status
    );

    if init {
        let capsule_volume_dir = cfg.capsule_volume_dir();
        Command::new("podman")
            .arg("exec")
            .arg("--user=root")
            .arg(format!("capsule-{}", container_id))
            .arg("bash")
            .arg(format!("{capsule_volume_dir}/.capsules/init.sh"))
            .spawn()
            .context("Failed to spawn init command")?
            .wait()
            .context("Failed to wait on init command")?;
    }
    Ok(())
}

pub fn delete_capsule(container_id: &str) -> Result<()> {
    let output = Command::new("podman")
        .arg("rm")
        .arg("-f")
        .arg(format!("capsule-{}", container_id))
        .output()
        .context("Failed to delete container")?;
    println!(
        "Container deleted: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

pub fn start_capsule(container_id: &str) -> Result<()> {
    let output = Command::new("podman")
        .arg("start")
        .arg(format!("capsule-{}", container_id))
        .output()
        .context("Failed to start container")?;
    println!(
        "Container started: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

pub fn stop_capsule(container_id: &str) -> Result<()> {
    let output = Command::new("podman")
        .arg("stop")
        .arg(format!("capsule-{}", container_id))
        .output()
        .context("Failed to stop container")?;
    println!(
        "Container stopped: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}
