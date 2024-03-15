use std::path::Path;

pub struct Runtime {}

pub struct ContainerId(pub String);

impl Runtime {
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#query-state>
    pub fn state(&self, _id: ContainerId) {}
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#create>
    pub fn create(&self, _id: ContainerId, _bundle_path: &Path) {}
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#start>
    pub fn start(&self, _id: ContainerId) {}
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#kill>
    pub fn kill(&self, _id: ContainerId, _signal: u8) {}
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#delete>
    pub fn delete(&self, _id: ContainerId) {}
}
