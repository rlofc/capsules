use std::path::Path;

use anyhow::{Context, Result};

use crate::{
    capsules::{CapsuleOptions, Capsules},
    tools::{ChildProcess, CommandWrapper, FileSystem},
    utils::CapsuleFile,
};

impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn create_a_new_capsule(
        &mut self,
        container_id: &str,
        additional_volumes: Vec<String>,
        init: bool,
        options: &CapsuleOptions,
    ) -> Result<()> {
        let exists = self
            .podman_cmd
            .reset()
            .args(&["container", "exists", &format!("capsule-{}", container_id)])
            .output()
            .context("Failed to check container existence")?;

        if exists.success {
            anyhow::bail!("Capsule '{}' already exists", container_id);
        }

        let capsule_username = self.fs.env_var("USER")?;

        let volumes_path = self.fs.current_dir()?;
        let home_path = volumes_path
            .join(self.cfg.capsule_home_dir())
            .join(&capsule_username);

        self.fs.create_dir_all(&home_path)?;

        let capsule_file = volumes_path.join(".capsules").join("capsule.toml");

        let contents = self.fs.read_to_string(&capsule_file)?;
        let capsule_desc: CapsuleFile =
            toml::from_str(&contents).context("Failed to parse TOML")?;
        let blueprint = capsule_desc
            .blueprint
            .context("Missing blueprint in capsule.toml")?;

        self.run_capsule_container(
            container_id,
            &blueprint,
            &volumes_path,
            &capsule_username,
            &additional_volumes,
            options,
        )?;

        if init {
            self.run_capsule_init(container_id)?;
        }
        Ok(())
    }

    /// The bind mounts every capsule gets regardless of `--volume`/`--no-pulse`,
    /// used to tell "extra" volumes a container already had apart from this scaffolding.
    pub(crate) fn default_run_volumes(&self, volumes_path: &Path) -> Vec<String> {
        let capsule_volume_dir = self.cfg.capsule_volume_dir();
        vec![
            "/dev/shm:/dev/shm:rw".to_string(),
            format!(
                "{}:{}:rw",
                volumes_path.to_str().unwrap(),
                capsule_volume_dir
            ),
            "/dev/snd:/dev/snd:rw".to_string(),
            "/run/user/1000/pulse:/run/user/host/pulse:rw".to_string(),
        ]
    }

    pub(crate) fn build_run_args(
        &mut self,
        container_id: &str,
        image: &str,
        volumes_path: &Path,
        capsule_username: &str,
        additional_volumes: &[String],
        options: &CapsuleOptions,
    ) -> Vec<String> {
        let volumes_path_as_str = volumes_path.to_str().unwrap().to_string();
        let capsule_volume_dir = self.cfg.capsule_volume_dir();
        let container_volume = format!("{volumes_path_as_str}:{capsule_volume_dir}:rw");
        let capsule_home_dir = self.cfg.capsule_home_dir();
        let display = self
            .fs
            .env_var("DISPLAY")
            .unwrap_or_else(|_| ":0".to_string());

        let mut args: Vec<String> = vec![
            "run".into(),
            "--init".into(),
            "-d".into(),
            "-h".into(),
            container_id.into(),
            "-e".into(),
            format!("DISPLAY={display}"),
            "--net=host".into(),
            "--userns=keep-id".into(),
            "--user=root".into(),
            "--pids-limit=-1".into(),
            "-v".into(),
            "/dev/shm:/dev/shm:rw".into(),
            "-v".into(),
            container_volume,
        ];

        if !options.no_gpu {
            args.push("--gpus".into());
            args.push("all".into());
        }

        if !options.no_pulse {
            args.push("-v".into());
            args.push("/dev/snd:/dev/snd:rw".into());
            args.push("-v".into());
            args.push("/run/user/1000/pulse:/run/user/host/pulse:rw".into());
        }

        for vol in additional_volumes {
            args.push("-v".into());
            args.push(vol.clone());
        }

        if !options.no_pulse {
            args.push("-e".into());
            args.push("PULSE_SERVER=unix:/run/user/host/pulse/native".into());
        }

        args.push("-e".into());
        args.push(format!("BOOTSTRAP={}.sh", container_id));
        args.push("-e".into());
        args.push(format!(
            "CAPSULE_HOMEDIR={capsule_volume_dir}/{capsule_home_dir}"
        ));
        args.push("-e".into());
        args.push(format!("CAPSULE_USERNAME={}", capsule_username));
        args.push("--name".into());
        args.push(format!("capsule-{}", container_id));
        args.push(image.to_string());
        args.push("tail".into());
        args.push("-f".into());
        args.push("/dev/null".into());

        args
    }

    pub(crate) fn execute_run_args(&mut self, args: &[String]) -> Result<()> {
        println!("Starting container...");
        let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self
            .podman_cmd
            .reset()
            .args(&arg_refs)
            .output()
            .context("Failed to execute podman run command")?;

        println!(
            "Container spun up: {} {}",
            output.stdout,
            if output.success { "success" } else { "failed" },
        );
        Ok(())
    }

    pub(crate) fn run_capsule_container(
        &mut self,
        container_id: &str,
        image: &str,
        volumes_path: &Path,
        capsule_username: &str,
        additional_volumes: &[String],
        options: &CapsuleOptions,
    ) -> Result<()> {
        let args = self.build_run_args(
            container_id,
            image,
            volumes_path,
            capsule_username,
            additional_volumes,
            options,
        );
        self.execute_run_args(&args)
    }

    pub(crate) fn run_capsule_init(&mut self, container_id: &str) -> Result<()> {
        println!("Running init script...");
        let capsule_volume_dir = self.cfg.capsule_volume_dir();
        self.podman_cmd
            .reset()
            .args(&["exec", "--user=root"])
            .args(&[&format!("capsule-{}", container_id)])
            .args(&["bash", &format!("{capsule_volume_dir}/.capsules/init.sh")])
            .spawn()
            .context("Failed to spawn init command")?
            .wait()
            .context("Failed to wait on init command")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        capsules::{CapsuleOptions, Capsules},
        tools::tests::{CommandWrapperMock, FileSystemMock, FsCall},
        utils::Config,
    };

    fn assert_run_command(cmd: &str) {
        assert!(cmd.starts_with("podman run --init -d -h foo -e DISPLAY=:99"));
        assert!(cmd.contains("--net=host --userns=keep-id --user=root --pids-limit=-1"));
        assert!(cmd.contains("-v /dev/shm:/dev/shm:rw"));
        assert!(cmd.contains("-v /mock/work:/files:rw"));
        assert!(cmd.contains("--gpus all"));
        assert!(cmd.contains("-v /dev/snd:/dev/snd:rw"));
        assert!(cmd.contains("-v /run/user/1000/pulse:/run/user/host/pulse:rw"));
        assert!(cmd.contains("-e PULSE_SERVER=unix:/run/user/host/pulse/native"));
        assert!(cmd.contains("-e BOOTSTRAP=foo.sh"));
        assert!(cmd.contains("-e CAPSULE_HOMEDIR=/files/home"));
        assert!(cmd.contains("-e CAPSULE_USERNAME=testuser"));
        assert!(cmd.contains("--name capsule-foo"));
        assert!(cmd.ends_with("vaxvms tail -f /dev/null"));
    }

    #[test]
    fn test_spin_a_new_capsule_with_init() {
        let fs_mock = FileSystemMock::new("vaxvms");
        let fs_log = fs_mock.log();
        let podman_mock = CommandWrapperMock::new("podman");
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: false,
            no_pulse: false,
        };

        let result = caps.create_a_new_capsule("foo", vec![], true, &options);

        assert!(result.is_ok(), "spin failed: {:?}", result.err());

        // Verify filesystem call sequence
        let calls = fs_log.borrow().clone();
        assert_eq!(calls.len(), 4);
        assert_eq!(calls[0], FsCall::EnvVar("USER".to_string()));
        assert_eq!(
            calls[1],
            FsCall::CreateDirAll(PathBuf::from("/mock/work/home/testuser"))
        );
        assert_eq!(
            calls[2],
            FsCall::ReadToString(PathBuf::from("/mock/work/.capsules/capsule.toml"))
        );
        assert_eq!(calls[3], FsCall::EnvVar("DISPLAY".to_string()));

        // Verify podman command history: exists check, run, then exec
        let log = podman_log.borrow();
        assert_eq!(log.len(), 3);
        assert_eq!(log[0], "podman container exists capsule-foo");
        assert_run_command(&log[1]);
        assert_eq!(
            log[2],
            "podman exec --user=root capsule-foo bash /files/.capsules/init.sh"
        );
    }

    #[test]
    fn test_spin_a_new_capsule_without_init() {
        let fs_mock = FileSystemMock::new("vaxvms");
        let _fs_log = fs_mock.log();
        let podman_mock = CommandWrapperMock::new("podman");
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: false,
            no_pulse: false,
        };

        let result = caps.create_a_new_capsule("foo", vec![], false, &options);

        assert!(result.is_ok(), "spin failed: {:?}", result.err());

        // Verify podman command history: exists check then run command
        let log = podman_log.borrow();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0], "podman container exists capsule-foo");
        assert_run_command(&log[1]);
    }
}
