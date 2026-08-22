use anyhow::{Context, Result};
use std::path::Path;

use crate::{
    capsules::{CapsuleOptions, Capsules},
    tools::{CommandWrapper, FileSystem},
};

impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn recreate_capsule(
        &mut self,
        container_id: &str,
        additional_volumes: Vec<String>,
        with_volumes: bool,
        debug: bool,
        options: &CapsuleOptions,
    ) -> Result<()> {
        let exists = self
            .podman_cmd
            .reset()
            .args(&["container", "exists", &format!("capsule-{}", container_id)])
            .output()
            .context("Failed to check container existence")?;

        if !exists.success {
            anyhow::bail!("Capsule '{}' does not exist", container_id);
        }

        let capsule_username = self.fs.env_var("USER")?;
        let volumes_path = self.fs.current_dir()?;

        let warning = format!(
            "You are about to recreate {} using \x1b[1m{}\x1b[0m as your volume [y/N] ",
            container_id,
            volumes_path.display()
        );
        if !self.fs.confirm(&warning)? {
            println!("Aborted.");
            return Ok(());
        }

        let mut additional_volumes = additional_volumes;
        if with_volumes {
            println!("Inspecting existing container volumes...");
            for vol in self.existing_container_volumes(container_id, &volumes_path)? {
                if !additional_volumes.contains(&vol) {
                    additional_volumes.push(vol);
                }
            }
        }

        let recreated_image = format!("capsule-{}-recreated", container_id);

        let run_args = self.build_run_args(
            container_id,
            &recreated_image,
            &volumes_path,
            &capsule_username,
            &additional_volumes,
            options,
        );

        // Confirm before touching the existing container: bailing out here leaves
        // it untouched, whereas confirming after the commit/rm would risk leaving
        // no container running at all if the user declined at the last step.
        if debug {
            println!("About to run: podman {}", run_args.join(" "));
            if !self.fs.confirm("Proceed? [y/N] ")? {
                println!("Aborted.");
                return Ok(());
            }
        }

        println!("Committing current container state...");
        self.podman_cmd
            .reset()
            .args(&[
                "commit",
                &format!("capsule-{}", container_id),
                &recreated_image,
            ])
            .output()
            .context("Failed to commit existing container")?;

        println!("Removing existing container...");
        self.podman_cmd
            .reset()
            .args(&["rm", "-f", &format!("capsule-{}", container_id)])
            .output()
            .context("Failed to remove existing container")?;

        // The recreated image is a commit of the already-initialized container,
        // so init.sh must not run again.
        self.execute_run_args(&run_args)?;

        Ok(())
    }

    fn existing_container_volumes(
        &mut self,
        container_id: &str,
        volumes_path: &Path,
    ) -> Result<Vec<String>> {
        let inspect = self
            .podman_cmd
            .reset()
            .args(&[
                "inspect",
                "--format",
                "{{range .Mounts}}{{if eq .Type \"bind\"}}{{.Source}}:{{.Destination}}:{{if .RW}}rw{{else}}ro{{end}}\n{{end}}{{end}}",
                &format!("capsule-{}", container_id),
            ])
            .output()
            .context("Failed to inspect existing container volumes")?;

        let defaults = self.default_run_volumes(volumes_path);

        Ok(inspect
            .stdout
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .filter(|l| !defaults.iter().any(|d| d == l))
            .map(str::to_string)
            .collect())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        capsules::{CapsuleOptions, Capsules},
        tools::tests::{CommandWrapperMock, FileSystemMock},
        utils::Config,
    };

    #[test]
    fn test_recreate_capsule_commits_removes_and_reruns() {
        let fs_mock = FileSystemMock::new("vaxvms").with_confirm(true);
        let podman_mock = CommandWrapperMock::new("podman").with_container_exists(true);
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: true,
            no_pulse: true,
        };

        let result = caps.recreate_capsule(
            "foo",
            vec!["/host:/container".to_string()],
            false,
            false,
            &options,
        );

        assert!(result.is_ok(), "recreate failed: {:?}", result.err());

        let log = podman_log.borrow();
        assert_eq!(log.len(), 4);
        assert_eq!(log[0], "podman container exists capsule-foo");
        assert_eq!(log[1], "podman commit capsule-foo capsule-foo-recreated");
        assert_eq!(log[2], "podman rm -f capsule-foo");
        assert!(log[3].starts_with("podman run --init -d -h foo -e DISPLAY=:99"));
        assert!(log[3].contains("-v /mock/work:/files:rw"));
        assert!(log[3].contains("-v /host:/container"));
        assert!(!log[3].contains("--gpus"));
        assert!(!log[3].contains("PULSE"));
        assert!(log[3].ends_with("capsule-foo-recreated tail -f /dev/null"));
    }

    #[test]
    fn test_recreate_capsule_fails_if_not_exists() {
        let podman_mock = CommandWrapperMock::new("podman");
        let mut caps = Capsules::new(Config::new(None, None)).with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: false,
            no_pulse: false,
        };

        let result = caps.recreate_capsule("foo", vec![], false, false, &options);

        assert!(result.is_err());
    }

    #[test]
    fn test_recreate_capsule_aborts_without_confirmation() {
        let fs_mock = FileSystemMock::new("vaxvms").with_confirm(false);
        let podman_mock = CommandWrapperMock::new("podman").with_container_exists(true);
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: false,
            no_pulse: false,
        };

        let result = caps.recreate_capsule("foo", vec![], false, false, &options);

        assert!(result.is_ok(), "recreate failed: {:?}", result.err());

        // Only the existence check ran; nothing destructive happened.
        let log = podman_log.borrow();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0], "podman container exists capsule-foo");
    }

    #[test]
    fn test_recreate_capsule_with_volumes_adds_existing_extras_only() {
        let fs_mock = FileSystemMock::new("vaxvms").with_confirm(true);
        let inspect_output = "/dev/shm:/dev/shm:rw\n\
             /mock/work:/files:rw\n\
             /dev/snd:/dev/snd:rw\n\
             /run/user/1000/pulse:/run/user/host/pulse:rw\n\
             /home/user/extra:/extra:rw\n";
        let podman_mock = CommandWrapperMock::new("podman")
            .with_container_exists(true)
            .with_inspect_output(inspect_output);
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: true,
            no_pulse: true,
        };

        let result = caps.recreate_capsule("foo", vec![], true, false, &options);

        assert!(result.is_ok(), "recreate failed: {:?}", result.err());

        let log = podman_log.borrow();
        assert_eq!(log.len(), 5);
        assert_eq!(
            log[1],
            "podman inspect --format {{range .Mounts}}{{if eq .Type \"bind\"}}{{.Source}}:{{.Destination}}:{{if .RW}}rw{{else}}ro{{end}}\n{{end}}{{end}} capsule-foo"
        );
        assert!(log[4].contains("-v /home/user/extra:/extra:rw"));
        assert_eq!(log[4].matches("-v ").count(), 3); // /dev/shm + main volume + the one extra
    }

    #[test]
    fn test_recreate_capsule_debug_aborts_without_confirmation() {
        let fs_mock = FileSystemMock::new("vaxvms").with_confirm(false);
        let podman_mock = CommandWrapperMock::new("podman").with_container_exists(true);
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: false,
            no_pulse: false,
        };

        let result = caps.recreate_capsule("foo", vec![], false, true, &options);

        assert!(result.is_ok(), "recreate failed: {:?}", result.err());

        // Only the existence check ran; nothing destructive happened.
        let log = podman_log.borrow();
        assert_eq!(log.len(), 1);
        assert_eq!(log[0], "podman container exists capsule-foo");
    }

    #[test]
    fn test_recreate_capsule_debug_proceeds_when_confirmed() {
        let fs_mock = FileSystemMock::new("vaxvms").with_confirm(true);
        let podman_mock = CommandWrapperMock::new("podman").with_container_exists(true);
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let options = CapsuleOptions {
            no_gpu: false,
            no_pulse: false,
        };

        let result = caps.recreate_capsule("foo", vec![], false, true, &options);

        assert!(result.is_ok(), "recreate failed: {:?}", result.err());

        let log = podman_log.borrow();
        assert_eq!(log.len(), 4);
        assert_eq!(log[0], "podman container exists capsule-foo");
        assert_eq!(log[1], "podman commit capsule-foo capsule-foo-recreated");
        assert_eq!(log[2], "podman rm -f capsule-foo");
        assert!(log[3].starts_with("podman run --init -d -h foo -e DISPLAY=:99"));
    }
}
