use anyhow::{Context, Result};

use crate::{
    capsules::Capsules,
    tools::{CommandWrapper, FileSystem},
};

impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn stop_capsule(&mut self, container_id: &str) -> Result<()> {
        let output = self
            .podman_cmd
            .reset()
            .args(&["stop", &format!("capsule-{}", container_id)])
            .output()
            .context("Failed to stop container")?;
        println!("Container stopped: {}", output.stdout);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{capsules::Capsules, tools::tests::CommandWrapperMock, utils::Config};

    #[test]
    fn test_stop_capsule_issues_correct_podman_command() {
        let mut caps =
            Capsules::new(Config::new(None, None)).with_podman(CommandWrapperMock::new("podman"));

        let result = caps.stop_capsule("foo");
        assert!(result.is_ok());

        assert_eq!(
            caps.podman_cmd_ref().command_line(),
            "podman stop capsule-foo"
        );
    }
}
