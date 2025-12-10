// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_std]
#![doc = include_str!("../README.md")]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]

pub mod protocol;
pub mod transport;

use thiserror::Error;

/// Rich error types returned by this module.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Invalid protocol ID {0}")]
    InvalidProtocolId(u8),
    #[error("Invalid message type {0}")]
    InvalidMessageType(u8),
    #[error("Invalid message ID {0}")]
    InvalidMessageId(u8),
    #[error("Invalid status code {0}")]
    InvalidStatusCode(i32),
}
