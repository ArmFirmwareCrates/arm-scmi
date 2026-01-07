// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::hint::spin_loop;

use bitflags::bitflags;
use safe_mmio::{UniqueMmioPointer, field, field_shared, fields::ReadPureWrite};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{
    Error,
    protocol::{Command, MessageHeader, StandardStatusCode, StatusCode},
    transport::Transport,
};

/// Shared memory channel status
///
/// See Table 33: Layout of the shared memory area
#[derive(Copy, Clone, Debug, Eq, FromBytes, Immutable, IntoBytes, KnownLayout, PartialEq)]
#[repr(transparent)]
pub struct ChannelStatus(u32);

bitflags! {
    impl ChannelStatus: u32 {
        /// Channel error
        const ERROR = 1 << 1;
        /// Channel free
        const FREE = 1 << 0;
    }
}

/// Shared memory channel flags
///
/// See Table 34: Channel flags
#[derive(Copy, Clone, Debug, Eq, FromBytes, Immutable, IntoBytes, KnownLayout, PartialEq)]
#[repr(transparent)]
pub struct ChannelFlags(u32);

bitflags! {
    impl ChannelFlags: u32 {
        /// The command should complete via an interrupt.
        const INTERRUPT = 1 << 0;
    }
}

/// Shared memory header structure
///
/// See Table 33: Layout of the shared memory area
#[derive(Clone, Eq, FromBytes, Immutable, IntoBytes, KnownLayout, PartialEq)]
#[repr(C, align(4))]
pub struct SharedMemoryHeader {
    reserved_0: [u8; 4],
    channel_status: ReadPureWrite<ChannelStatus>,
    reserved_8: [u8; 8],
    channel_flags: ReadPureWrite<ChannelFlags>,
    length: ReadPureWrite<u32>,
    message_header: ReadPureWrite<u32>,
}

//#[derive(Clone, Eq, FromBytes, Immutable, IntoBytes, KnownLayout, PartialEq)]
#[repr(C, align(4))]
pub struct SharedMemoryLayout<const N: usize> {
    header: SharedMemoryHeader,
    payload: ReadPureWrite<[u8; N]>,
}

/// Doorbell interface.
pub trait Doorbell {
    /// Rings the doorbell.
    fn ring(&self);
}

/// Shared memory based transport implementation.
pub struct SharedMemoryTransport<'a, const LEN: usize, D: Doorbell> {
    memory: UniqueMmioPointer<'a, SharedMemoryLayout<LEN>>,
    doorbell: D,
}

impl<'a, const LEN: usize, D: Doorbell> SharedMemoryTransport<'a, LEN, D> {
    /// Creates new instance.
    pub fn new(memory: UniqueMmioPointer<'a, SharedMemoryLayout<LEN>>, doorbell: D) -> Self {
        Self { memory, doorbell }
    }

    /// Writes command into the shared memory.
    fn write_command<C: Command>(&mut self, command: C) {
        const {
            assert!(size_of::<C>() <= LEN);
        }

        // Safety: The pointer is owned by this object and points to a valid, writetable memory.
        // Concurrent access between the AP and the platform is prevented by the doorbell.
        unsafe {
            core::ptr::write_volatile(field!(self.memory, payload).ptr_mut() as *mut C, command);
        }
    }

    /// Reads response from the shared memory.
    fn read_response<R>(&self) -> Result<R, Error> {
        const {
            assert!(size_of::<R>() <= LEN);
        }

        #[repr(C)]
        struct Response<R> {
            code: i32,
            payload: R,
        }

        // Safety: The pointer is owned by this object and points to a valid, readable memory.
        // Concurrent access between the AP and the platform is prevented by the doorbell.
        let Response { code, payload } = unsafe {
            core::ptr::read_volatile(field_shared!(self.memory, payload).ptr() as *const _)
        };

        let code = code.try_into()?;
        if code != StatusCode::Standard(StandardStatusCode::Success) {
            return Err(Error::Status(code));
        }

        Ok(payload)
    }

    /// Sets the channel to busy.
    pub fn set_busy(&mut self) {
        let mut header = field!(self.memory, header);
        let mut status = field!(header, channel_status);

        status.write(status.read() - ChannelStatus::FREE);
    }

    /// Checks if the channel is free.
    pub fn is_free(&self) -> bool {
        let header = field_shared!(self.memory, header);
        let status = field_shared!(header, channel_status);

        status.read().contains(ChannelStatus::FREE)
    }
}

impl<'a, const LEN: usize, D: Doorbell> Transport for SharedMemoryTransport<'a, LEN, D> {
    fn send_sync_command<C: Command>(&mut self, command: C) -> Result<C::Response, Error> {
        while !self.is_free() {
            spin_loop();
        }

        let mut header = field!(self.memory, header);
        field!(header, message_header).write(
            MessageHeader {
                token: 0,
                message_id: C::ID,
            }
            .into(),
        );

        self.write_command(command);
        self.set_busy();

        self.doorbell.ring();

        while !self.is_free() {
            spin_loop();
        }

        self.read_response()
    }
}
