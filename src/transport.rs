// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod shared_memory;

use crate::{Error, protocol::Command};

/// Trait for implementing different transport methods.
///
/// TODO: identify common steps of different transport types.
pub trait Transport {
    fn send_sync_command<C: Command>(&mut self, command: C) -> Result<C::Response, Error>;
}
