use eyre::Result;
use std::{
    collections::HashMap,
    convert::Infallible,
    fmt::{Debug, Display},
    path::{Path, PathBuf},
    str::FromStr,
};

pub struct Runtime {}

#[derive(Clone)]
pub struct ContainerId(pub String);

#[derive(Debug)]
pub struct State {
    pub oci_version: String,
    pub id: ContainerId,
    pub status: Status,
    pub pid: Option<i64>,
    pub bundle: PathBuf,
    pub annotations: HashMap<String, String>,
}

#[derive(Debug)]
pub enum Status {
    Creating,
    Created,
    Running,
    Stopped,
}

impl Runtime {
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#query-state>
    pub fn state(&self, _id: ContainerId) -> Result<State> {
        todo!()
    }
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#create>
    pub fn create(&self, _id: ContainerId, _bundle_path: &Path) -> Result<()> {
        todo!()
    }
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#start>
    pub fn start(&self, _id: ContainerId) -> Result<()> {
        todo!()
    }
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#kill>
    pub fn kill(&self, _id: ContainerId, _signal: u8) -> Result<()> {
        todo!()
    }
    /// <https://github.com/opencontainers/runtime-spec/blob/main/runtime.md#delete>
    pub fn delete(&self, _id: ContainerId) -> Result<()> {
        todo!()
    }
}

impl FromStr for ContainerId {
    type Err = Infallible;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        Ok(Self(s.to_owned()))
    }
}

impl Debug for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Display for ContainerId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
