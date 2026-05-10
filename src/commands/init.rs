use anyhow::{Context, Result};
use std::process::Stdio;

use crate::{
    capsules::Capsules,
    tools::{ChildProcess, CommandWrapper, FileSystem},
};

impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn init_container_volume(&mut self, blueprint: &str) -> Result<()> {
        let source_path = self.fs.user_config_folder().join(blueprint);
        if !self.fs.path_exists(&source_path) {
            anyhow::bail!("Source path does not exist: {}", source_path.display());
        }

        let volumes_path = self.fs.current_dir()?;
        let capsule_dir = volumes_path.join(".capsules");

        if self.fs.path_exists(&capsule_dir) {
            anyhow::bail!(".capsules already exists in the current directory");
        }

        self.fs.copy_dir(&source_path, &capsule_dir)?;

        let mut tar = self
            .tar_cmd
            .args(&["-czh", "."])
            .current_dir(&capsule_dir)
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to spawn tar")?;

        let mut podman = self
            .podman_cmd
            .args(&["build", "-t", blueprint, "-"])
            .stdin(tar.take_stdout().unwrap())
            .spawn()
            .context("Failed to spawn podman build")?;

        if !podman.wait()? {
            anyhow::bail!("podman build failed");
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{
        capsules::Capsules,
        tools::tests::{CommandWrapperMock, FileSystemMock, FsCall},
        utils::Config,
    };

    #[test]
    fn test_init_uses_correct_filesystem_and_command_calls() {
        let fs_mock = FileSystemMock::new("os370");
        let fs_log = fs_mock.log();
        let mut caps = Capsules::new(Config::new(None, None))
            .with_fs(fs_mock)
            .with_tar(CommandWrapperMock::new("tar"))
            .with_podman(CommandWrapperMock::new("podman"));

        let result = caps.init_container_volume("os370");

        assert!(result.is_ok());

        let calls = fs_log.borrow().clone();
        assert_eq!(calls.len(), 3);
        assert_eq!(
            calls[0],
            FsCall::PathExists(PathBuf::from("/mock/home/.config/capsules/os370"))
        );
        assert_eq!(
            calls[1],
            FsCall::PathExists(PathBuf::from("/mock/work/.capsules"))
        );
        assert_eq!(
            calls[2],
            FsCall::CopyDir(
                PathBuf::from("/mock/home/.config/capsules/os370"),
                PathBuf::from("/mock/work/.capsules"),
            )
        );

        let tar_line = caps.tar_cmd_ref().command_line();
        assert!(tar_line.starts_with("tar -czh . (cwd: "));
        assert!(tar_line.ends_with("/.capsules)"));
        assert_eq!(
            caps.podman_cmd_ref().command_line(),
            "podman build -t os370 -"
        );
    }
}
