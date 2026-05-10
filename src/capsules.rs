use crate::tools::{CommandWrapper, CommandWrapperImpl, FileSystem, FileSystemImpl};
use crate::utils::Config;

#[allow(dead_code)]
pub struct Capsules<F: FileSystem, T: CommandWrapper, P: CommandWrapper> {
    pub cfg: Config,
    pub fs: F,
    pub tar_cmd: T,
    pub podman_cmd: P,
}

impl Capsules<FileSystemImpl, CommandWrapperImpl, CommandWrapperImpl> {
    pub fn new(cfg: Config) -> Self {
        Self {
            cfg,
            fs: FileSystemImpl,
            tar_cmd: CommandWrapperImpl::new("tar"),
            podman_cmd: CommandWrapperImpl::new("podman"),
        }
    }
}

#[allow(dead_code)]
impl<F: FileSystem, T: CommandWrapper, P: CommandWrapper> Capsules<F, T, P> {
    pub fn with_fs<F2: FileSystem>(self, fs: F2) -> Capsules<F2, T, P> {
        Capsules {
            cfg: self.cfg,
            fs,
            tar_cmd: self.tar_cmd,
            podman_cmd: self.podman_cmd,
        }
    }
    pub fn with_tar<T2: CommandWrapper>(self, tar_cmd: T2) -> Capsules<F, T2, P> {
        Capsules {
            cfg: self.cfg,
            fs: self.fs,
            tar_cmd,
            podman_cmd: self.podman_cmd,
        }
    }
    pub fn with_podman<P2: CommandWrapper>(self, podman_cmd: P2) -> Capsules<F, T, P2> {
        Capsules {
            cfg: self.cfg,
            fs: self.fs,
            tar_cmd: self.tar_cmd,
            podman_cmd,
        }
    }

    pub fn fs_ref(&self) -> &F {
        &self.fs
    }
    pub fn tar_cmd_ref(&self) -> &T {
        &self.tar_cmd
    }
    pub fn podman_cmd_ref(&self) -> &P {
        &self.podman_cmd
    }
}

pub struct CapsuleOptions {
    pub no_gpu: bool,
    pub no_pulse: bool,
}
