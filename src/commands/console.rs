use anyhow::{Context, Result};

use crate::{
    capsules::Capsules,
    tools::{ChildProcess, CommandWrapper, FileSystem},
};

impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn capsule_console_as_root(
        &mut self,
        container_id: &str,
        command: Option<String>,
    ) -> Result<()> {
        let cmd = command.unwrap_or_else(|| "sh".to_string());
        self.podman_cmd
            .reset()
            .args(&["exec", "-it", "--user=root"])
            .args(&[&format!("capsule-{}", container_id)])
            .args(&[&cmd])
            .spawn()
            .context("Failed to spawn console command")?
            .wait()
            .context("Failed to wait on console command")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{capsules::Capsules, tools::tests::CommandWrapperMock, utils::Config};

    #[test]
    fn test_console_without_command_uses_sh() {
        let mut caps =
            Capsules::new(Config::new(None, None)).with_podman(CommandWrapperMock::new("podman"));

        let result = caps.capsule_console_as_root("bar", None);
        assert!(result.is_ok());

        assert_eq!(
            caps.podman_cmd_ref().command_line(),
            "podman exec -it --user=root capsule-bar sh"
        );
    }

    #[test]
    fn test_console_with_custom_command() {
        let mut caps =
            Capsules::new(Config::new(None, None)).with_podman(CommandWrapperMock::new("podman"));

        let result = caps.capsule_console_as_root("bar", Some("bash".to_string()));
        assert!(result.is_ok());

        assert_eq!(
            caps.podman_cmd_ref().command_line(),
            "podman exec -it --user=root capsule-bar bash"
        );
    }
}
