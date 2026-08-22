use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::{env, fs};

use anyhow::{Context, Result};

use crate::utils::{copy_directory, get_user_config_folder};

pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
}

pub trait CommandWrapper {
    type Process: ChildProcess;
    fn args(&mut self, args: &[&str]) -> &mut Self;
    fn current_dir(&mut self, dir: &Path) -> &mut Self;
    fn stdout(&mut self, cfg: Stdio) -> &mut Self;
    fn stdin(&mut self, cfg: Stdio) -> &mut Self;
    fn spawn(&mut self) -> Result<Self::Process>;
    fn output(&mut self) -> Result<CommandOutput>;
    fn reset(&mut self) -> &mut Self;
}

pub trait ChildProcess {
    fn take_stdout(&mut self) -> Option<Stdio>;
    fn wait(&mut self) -> Result<bool>;
}

pub struct CommandWrapperImpl {
    program: String,
    command: Command,
}

impl CommandWrapperImpl {
    pub fn new(program: &str) -> Self {
        CommandWrapperImpl {
            program: program.to_string(),
            command: Command::new(program),
        }
    }
}

impl CommandWrapper for CommandWrapperImpl {
    type Process = RealChild;
    fn args(&mut self, args: &[&str]) -> &mut Self {
        self.command.args(args);
        self
    }
    fn current_dir(&mut self, dir: &Path) -> &mut Self {
        self.command.current_dir(dir);
        self
    }
    fn stdout(&mut self, cfg: Stdio) -> &mut Self {
        self.command.stdout(cfg);
        self
    }
    fn stdin(&mut self, cfg: Stdio) -> &mut Self {
        self.command.stdin(cfg);
        self
    }
    fn spawn(&mut self) -> Result<Self::Process> {
        Ok(RealChild(self.command.spawn()?))
    }
    fn output(&mut self) -> Result<CommandOutput> {
        let output = self.command.output()?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        })
    }
    fn reset(&mut self) -> &mut Self {
        self.command = Command::new(&self.program);
        self
    }
}

pub struct RealChild(Child);

impl ChildProcess for RealChild {
    fn take_stdout(&mut self) -> Option<Stdio> {
        self.0.stdout.take().map(|s| s.into())
    }
    fn wait(&mut self) -> Result<bool> {
        Ok(self.0.wait()?.success())
    }
}

pub trait FileSystem {
    fn user_config_folder(&self) -> PathBuf;
    fn path_exists(&self, path: &Path) -> bool;
    fn current_dir(&self) -> Result<PathBuf>;
    fn copy_dir(&self, src: &Path, dst: &Path) -> Result<()>;
    fn create_dir_all(&self, path: &Path) -> Result<()>;
    fn read_to_string(&self, path: &Path) -> Result<String>;
    fn env_var(&self, key: &str) -> Result<String>;
    fn confirm(&self, prompt: &str) -> Result<bool>;
}

pub struct FileSystemImpl;

impl FileSystem for FileSystemImpl {
    fn user_config_folder(&self) -> PathBuf {
        get_user_config_folder()
    }
    fn path_exists(&self, path: &Path) -> bool {
        path.exists()
    }
    fn current_dir(&self) -> Result<PathBuf> {
        env::current_dir().context("Failed to get current directory")
    }
    fn copy_dir(&self, src: &Path, dst: &Path) -> Result<()> {
        copy_directory(src.to_str().unwrap(), dst.to_str().unwrap())
            .context("Failed to copy bootstrap script")
    }
    fn create_dir_all(&self, path: &Path) -> Result<()> {
        fs::create_dir_all(path).context("Failed to create directory")
    }
    fn read_to_string(&self, path: &Path) -> Result<String> {
        fs::read_to_string(path).with_context(|| format!("Failed to read {:?}", path))
    }
    fn env_var(&self, key: &str) -> Result<String> {
        env::var(key).with_context(|| format!("${key} is not set or cannot be used"))
    }
    fn confirm(&self, prompt: &str) -> Result<bool> {
        use std::io::Write;
        print!("{prompt}");
        std::io::stdout().flush().ok();
        let mut input = String::new();
        std::io::stdin()
            .read_line(&mut input)
            .context("Failed to read confirmation")?;
        Ok(input.trim().eq_ignore_ascii_case("y"))
    }
}

#[cfg(test)]
pub mod tests {
    use crate::tools::{ChildProcess, CommandOutput};

    use super::*;
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};
    use std::process::Stdio;
    use std::rc::Rc;

    pub type CallLog = Rc<RefCell<Vec<FsCall>>>;

    #[derive(Debug, Clone, PartialEq)]
    pub enum FsCall {
        PathExists(PathBuf),
        CopyDir(PathBuf, PathBuf),
        CreateDirAll(PathBuf),
        ReadToString(PathBuf),
        EnvVar(String),
    }

    pub struct FileSystemMock {
        log: CallLog,
        blueprint: String,
        blueprint_exists: bool,
        capsules_exist: bool,
        capsule_toml: String,
        env_values: std::collections::HashMap<String, String>,
        confirm_answer: bool,
    }

    impl FileSystemMock {
        pub fn new(blueprint: &str) -> Self {
            let mut env_values = std::collections::HashMap::new();
            env_values.insert("USER".to_string(), "testuser".to_string());
            env_values.insert("DISPLAY".to_string(), ":99".to_string());
            FileSystemMock {
                log: Rc::new(RefCell::new(vec![])),
                blueprint: blueprint.to_string(),
                blueprint_exists: true,
                capsules_exist: false,
                capsule_toml: format!("blueprint = \"{}\"", blueprint),
                env_values,
                confirm_answer: false,
            }
        }
        pub fn with_confirm(mut self, answer: bool) -> Self {
            self.confirm_answer = answer;
            self
        }
        pub fn log(&self) -> CallLog {
            self.log.clone()
        }
    }

    impl FileSystem for FileSystemMock {
        fn user_config_folder(&self) -> PathBuf {
            PathBuf::from("/mock/home/.config/capsules")
        }
        fn path_exists(&self, path: &Path) -> bool {
            self.log
                .borrow_mut()
                .push(FsCall::PathExists(path.to_path_buf()));
            if path.ends_with(&self.blueprint) {
                self.blueprint_exists
            } else if path.ends_with(".capsules") {
                self.capsules_exist
            } else {
                false
            }
        }
        fn current_dir(&self) -> Result<PathBuf> {
            Ok(PathBuf::from("/mock/work"))
        }
        fn copy_dir(&self, src: &Path, dst: &Path) -> Result<()> {
            self.log
                .borrow_mut()
                .push(FsCall::CopyDir(src.to_path_buf(), dst.to_path_buf()));
            Ok(())
        }
        fn create_dir_all(&self, path: &Path) -> Result<()> {
            self.log
                .borrow_mut()
                .push(FsCall::CreateDirAll(path.to_path_buf()));
            Ok(())
        }
        fn read_to_string(&self, path: &Path) -> Result<String> {
            self.log
                .borrow_mut()
                .push(FsCall::ReadToString(path.to_path_buf()));
            Ok(self.capsule_toml.clone())
        }
        fn env_var(&self, key: &str) -> Result<String> {
            self.log.borrow_mut().push(FsCall::EnvVar(key.to_string()));
            self.env_values
                .get(key)
                .cloned()
                .ok_or_else(|| anyhow::anyhow!("${} not set", key))
        }
        fn confirm(&self, _prompt: &str) -> Result<bool> {
            Ok(self.confirm_answer)
        }
    }

    pub struct CommandWrapperMock {
        program: String,
        args: Vec<String>,
        dir: Option<String>,
        container_exists: bool,
        inspect_output: String,
        command_log: Rc<RefCell<Vec<String>>>,
    }

    impl CommandWrapperMock {
        pub fn new(program: &str) -> Self {
            CommandWrapperMock {
                program: program.to_string(),
                args: vec![],
                dir: None,
                container_exists: false,
                inspect_output: String::new(),
                command_log: Rc::new(RefCell::new(vec![])),
            }
        }
        pub fn with_container_exists(mut self, exists: bool) -> Self {
            self.container_exists = exists;
            self
        }
        pub fn with_inspect_output(mut self, output: &str) -> Self {
            self.inspect_output = output.to_string();
            self
        }
        pub fn command_log(&self) -> Rc<RefCell<Vec<String>>> {
            self.command_log.clone()
        }
        pub fn command_line(&self) -> String {
            let mut s = self.program.clone();
            for arg in &self.args {
                s.push(' ');
                s.push_str(arg);
            }
            if let Some(dir) = &self.dir {
                s.push_str(&format!(" (cwd: {dir})"));
            }
            s
        }
    }

    impl CommandWrapper for CommandWrapperMock {
        type Process = MockChild;
        fn args(&mut self, args: &[&str]) -> &mut Self {
            self.args.extend(args.iter().map(|s| s.to_string()));
            self
        }
        fn current_dir(&mut self, dir: &Path) -> &mut Self {
            self.dir = Some(dir.display().to_string());
            self
        }
        fn stdout(&mut self, _cfg: Stdio) -> &mut Self {
            self
        }
        fn stdin(&mut self, _cfg: Stdio) -> &mut Self {
            self
        }
        fn spawn(&mut self) -> Result<Self::Process> {
            self.command_log.borrow_mut().push(self.command_line());
            Ok(MockChild)
        }
        fn output(&mut self) -> Result<CommandOutput> {
            self.command_log.borrow_mut().push(self.command_line());
            let exists_check = self.args.iter().any(|a| a == "exists");
            let inspect_check = self.args.iter().any(|a| a == "inspect");
            Ok(CommandOutput {
                success: if exists_check {
                    self.container_exists
                } else {
                    true
                },
                stdout: if inspect_check {
                    self.inspect_output.clone()
                } else {
                    String::new()
                },
            })
        }
        fn reset(&mut self) -> &mut Self {
            self.args.clear();
            self.dir = None;
            self
        }
    }

    pub struct MockChild;

    impl ChildProcess for MockChild {
        fn take_stdout(&mut self) -> Option<Stdio> {
            Some(Stdio::null())
        }
        fn wait(&mut self) -> Result<bool> {
            Ok(true)
        }
    }
}
