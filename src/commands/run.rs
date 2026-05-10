use anyhow::{Context, Result};

use crate::{
    capsules::Capsules,
    tools::{ChildProcess, CommandWrapper, FileSystem},
};

impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn execute_in_capsule(
        &mut self,
        container_id: &str,
        command: &str,
        args: Vec<String>,
    ) -> Result<()> {
        self.podman_cmd
            .reset()
            .args(&["start", &format!("capsule-{}", container_id)])
            .output()
            .context("Failed to start container")?;

        let username_str = self.fs.env_var("USER")?;

        self.podman_cmd
            .reset()
            .args(&["exec", "-it", "--user", username_str.trim()])
            .args(&[&format!("capsule-{}", container_id)])
            .args(&[command]);

        for arg in &args {
            self.podman_cmd.args(&[arg.as_str()]);
        }

        self.podman_cmd
            .spawn()
            .context("Failed to execute command")?
            .wait()
            .context("Failed to wait on command")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        capsules::Capsules,
        tools::tests::{CommandWrapperMock, FileSystemMock},
        utils::Config,
    };

    #[test]
    fn test_execute_in_capsule_issues_correct_commands() {
        let fs_mock = FileSystemMock::new("os2");
        let podman_mock = CommandWrapperMock::new("podman");
        let podman_log = podman_mock.command_log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_podman(podman_mock);

        let result = caps.execute_in_capsule("foo", "ls", vec!["-la".to_string()]);
        assert!(result.is_ok());

        let log = podman_log.borrow();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0], "podman start capsule-foo");
        assert_eq!(log[1], "podman exec -it --user testuser capsule-foo ls -la");
    }
}
