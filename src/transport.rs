// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Shared memory and doorbell based transport implementation.
pub mod shared_memory;

use crate::{Error, protocol::Command};

/// Trait for implementing SCMI transport layers.
pub trait Transport {
    fn invoke_command<C: Command>(&mut self, command: C) -> Result<C::Response, Error>;
}
