use anyhow::{Context, Result};

use crate::{
    capsules::Capsules,
    tools::{CommandWrapper, FileSystem},
};

impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn list_capsules(&mut self) -> Result<()> {
        let output = self
            .podman_cmd
            .reset()
            .args(&[
                "container",
                "list",
                "-a",
                "--format",
                "{{printf \"% -30s %-60s %-40s\" .Names .Image .Status}}",
            ])
            .output()
            .context("Failed to list containers")?;

        for line in output.stdout.lines() {
            if line.contains("capsule") {
                println!("{}", line);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{capsules::Capsules, tools::tests::CommandWrapperMock, utils::Config};

    #[test]
    fn test_list_capsules_issues_correct_podman_command() {
        let mut caps =
            Capsules::new(Config::new(None, None)).with_podman(CommandWrapperMock::new("podman"));

        let result = caps.list_capsules();
        assert!(result.is_ok());

        assert_eq!(
            caps.podman_cmd_ref().command_line(),
            "podman container list -a --format {{printf \"% -30s %-60s %-40s\" .Names .Image .Status}}"
        );
    }
}
