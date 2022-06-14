use jotta::path::{PathOnDevice, UserScopedPath};

mod chunk;

pub(crate) type Result<T> = core::result::Result<T, jotta::Error>;

pub(crate) const DEVICE: &str = "Jotta";
pub(crate) const MOUNT_POINT: &str = "Archive";

pub struct Context<S> {
    client: jotta::Client<S>,
    root_folder: String,
}

impl<S> Context<S> {
    pub(crate) fn user_root(&self) -> UserScopedPath {
        UserScopedPath(format!("{DEVICE}/{MOUNT_POINT}/{}", self.root_folder))
    }

    pub(crate) fn device_root(&self) -> PathOnDevice {
        PathOnDevice(format!("{MOUNT_POINT}/{}", self.root_folder))
    }
}
