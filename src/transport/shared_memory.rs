// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Error,
    protocol::{
        Command, MessageHeader, MessageId, ResponseWithStatus, StandardStatusCode, StatusCode,
    },
    transport::Transport,
};
use bitflags::bitflags;
use core::{
    hint::spin_loop,
    ptr::{NonNull, slice_from_raw_parts_mut},
};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

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
#[derive(Clone, Debug, Eq, FromBytes, Immutable, IntoBytes, KnownLayout, PartialEq)]
#[repr(C, align(4))]
pub struct SharedMemoryHeader {
    reserved_0: u32,
    channel_status: ChannelStatus,
    // This member is defined as an u32 array because the header is only 4 bytes aligned.
    reserved_8: [u32; 2],
    channel_flags: ChannelFlags,
    length: u32,
    message_header: u32,
}

impl SharedMemoryHeader {
    /// Creates an empty header, marking the channel as free and configures the command such that it
    /// won't raise an an interrupt when complete.
    pub fn new_free() -> Self {
        Self {
            reserved_0: 0,
            channel_status: ChannelStatus::FREE,
            reserved_8: [0; 2],
            channel_flags: ChannelFlags::empty(),
            length: size_of::<u32>() as u32, // Message header length.
            message_header: 0,
        }
    }

    /// Creates a new instance that sets the passed fields and marks the channel busy.
    pub fn new_mark_busy(message_id: MessageId, length: u32, token: u16) -> Self {
        // The message must be at least as long as the message header.
        assert!(length as usize >= size_of::<u32>());

        Self {
            reserved_0: 0,
            channel_status: ChannelStatus::empty(), // Marks the channel busy.
            reserved_8: [0; 2],
            channel_flags: ChannelFlags::empty(),
            length,
            message_header: MessageHeader { token, message_id }.into(),
        }
    }

    /// Returns message header field or error if the message is too short or the field is invalid.
    pub fn message_header(&self) -> Result<MessageHeader, Error> {
        if self.length as usize >= size_of::<u32>() {
            MessageHeader::try_from(self.message_header)
        } else {
            Err(Error::ResponseTooShort)
        }
    }
}

/// SCMI shared memory component that implements access to the shared memory fields.
#[derive(Debug)]
pub struct SharedMemory {
    header: NonNull<SharedMemoryHeader>,
    payload: NonNull<[u32]>,
}

impl SharedMemory {
    /// Creates new SCMI shared memory from its start pointer and its size in bytes.
    ///
    /// # Safety
    ///
    /// `memory` must be a valid and unique pointer to a SCMI shared memory region that is
    /// accessible from all cores/threads. It must be at least 4 byte aligned and mapped as device
    /// memory. The shared memory must be at least 24 bytes + [maximal expected payload length] long.
    pub unsafe fn new(memory: NonNull<u32>, size: usize) -> Self {
        const HEADER_SIZE: usize = size_of::<SharedMemoryHeader>();
        // The payload should be at least one word long.
        const MIN_SIZE: usize = HEADER_SIZE + size_of::<u32>();

        assert!(size >= MIN_SIZE);
        assert!(memory.is_aligned());
        assert!(size.is_multiple_of(size_of::<u32>()));

        let header = memory.cast::<SharedMemoryHeader>();

        // Safety: header is a valid SharedMemoryHeader pointer.
        unsafe {
            header.write_volatile(SharedMemoryHeader::new_free());
        }

        // Safety: memory is a valid and aligned pointer and points to a memory that is at least
        // MIN_SIZE long.
        let payload_start = unsafe { memory.byte_add(HEADER_SIZE) };

        let payload = NonNull::new(slice_from_raw_parts_mut(
            payload_start.as_ptr(),
            (size - HEADER_SIZE) / size_of::<u32>(),
        ))
        .unwrap();

        Self { header, payload }
    }

    /// Reads channel status field of the shared memory header.
    pub fn channel_status(&self) -> ChannelStatus {
        // Safety: Self::new promises that self.header is a valid and aligned SharedMemoryHeader
        // pointer.
        let ptr = unsafe { &raw const (*self.header.as_ptr()).channel_status };

        // Safety: ptr is constructed from a valid SharedMemoryHeader pointer, thus it points to a
        // valid channel_status field. It is valid to read the status flags, the SCMI platform
        // changes the status in a single step.
        unsafe { ptr.read_volatile() }
    }

    /// Writes the channel status of the shared memory header.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that it owns the shared memory, i.e. the channel is 'free'.
    pub unsafe fn set_channel_status(&mut self, channel_status: ChannelStatus) {
        // Safety: Self::new promises that self.header is a valid and aligned SharedMemoryHeader
        // pointer.
        let ptr = unsafe { &raw mut (*self.header.as_ptr()).channel_status };

        // Safety: ptr is constructed from a valid SharedMemoryHeader pointer, thus it points to a
        // valid channel_status field. The caller guarantees that the it owns the shared memory,
        // preventing concurrent writes.
        unsafe {
            ptr.write_volatile(channel_status);
        }
    }

    /// Read the shared memory header.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that it owns the shared memory, i.e. the channel is 'free'.
    pub unsafe fn shared_memory_header(&self) -> SharedMemoryHeader {
        // Safety: self.header is promised to be valid by Self::new. The caller guarantees that
        // the channel is free and there are no concurrent writes to the shared memory.
        unsafe { self.header.read_volatile() }
    }

    /// Writes the shared memory header.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that it owns the shared memory, i.e. the channel is 'free'.
    pub unsafe fn set_shared_memory_header(&mut self, header: SharedMemoryHeader) {
        // Safety: self.header is promised to be valid by Self::new. The caller guarantees that
        // the channel is free and there are no concurrent writes to the shared memory.
        unsafe {
            self.header.write_volatile(header);
        }
    }

    /// Reads the payload from the shared memory and interprets it as `T`.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that it owns the shared memory, i.e. the channel is 'free'.
    pub unsafe fn payload<T>(&self) -> T {
        const {
            assert!(align_of::<T>() <= 4);
        };
        assert!(size_of::<T>() <= self.max_payload_length());

        let ptr = self.payload.cast::<T>();

        // Safety: Self::new promises that self.payload points to valid, aligned shared memory,
        // and the caller ensures the payload is initialized as a `T`.
        unsafe { ptr.read_volatile() }
    }

    /// Writes the payload into the shared memory.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that it owns the shared memory, i.e. the channel is 'free'.
    pub unsafe fn set_payload<T>(&mut self, payload: T) {
        const {
            assert!(align_of::<T>() <= 4);
        };
        assert!(size_of::<T>() <= self.max_payload_length());

        let ptr = self.payload.cast::<T>();

        // Safety: Self::new promises that self.payload points to valid, aligned shared memory,
        // and the caller ensures the payload is initialized as a `T`.
        unsafe { ptr.write_volatile(payload) };
    }

    /// Returns maximal possible payload size in bytes.
    pub fn max_payload_length(&self) -> usize {
        self.payload.len() * size_of::<u32>()
    }
}

// Safety: SharedMemory::new promises the memory to be accessible from all cores.
unsafe impl Send for SharedMemory {}

/// `OwnedChannel` is only constructed if the channel is free, making accesses to the shared memory
/// safe.
struct OwnedChannel<'a> {
    memory: &'a mut SharedMemory,
}

impl<'a> OwnedChannel<'a> {
    const TOKEN: u16 = 0;

    /// Waits for the channel to become free and creates a new instance. This ensures that the
    /// shared memory is safe to access, because there's no concurrent access to it.
    pub fn new(memory: &'a mut SharedMemory) -> Self {
        while !memory.channel_status().contains(ChannelStatus::FREE) {
            spin_loop();
        }

        Self { memory }
    }

    /// Writes command into the shared memory.
    ///
    /// Marks the channel to busy and consumes the object, because it is no longer owned.
    pub fn write_command<C: Command>(self, command: C) -> Result<(), Error> {
        let mut length = u32::try_from(size_of::<C>()).map_err(|_| Error::LengthOverflow)?;

        if length as usize > self.memory.max_payload_length() {
            return Err(Error::PayloadExceedsMaxSize);
        }

        // The length includes the length of the message_id too.
        length = length
            .checked_add(size_of::<u32>() as u32)
            .ok_or(Error::LengthOverflow)?;

        let header = SharedMemoryHeader::new_mark_busy(C::ID, length, Self::TOKEN);

        // Safety: Self::new guarantees that the channel is owned by the caller.
        unsafe {
            self.memory.set_payload(command);

            // Setting the shared memory header after writing the payload, because it sets the busy
            // flag.
            self.memory.set_shared_memory_header(header);
        };

        Ok(())
    }

    /// Reads response from the shared memory.
    fn read_response<C: Command>(&self) -> Result<C::Response, Error> {
        // Safety: Self::new guarantees that the channel is owned by the caller.
        let header = unsafe { self.memory.shared_memory_header() };

        // The length include the message_id too.
        let length = header
            .length
            .checked_sub(size_of::<u32>() as u32)
            .ok_or(Error::LengthOverflow)?;

        if length as usize > self.memory.max_payload_length()
            || (length as usize) != size_of::<ResponseWithStatus<C::Response>>()
        {
            return Err(Error::PayloadExceedsMaxSize);
        }

        let msg_header = header.message_header()?;
        if msg_header.message_id != C::ID {
            return Err(Error::UnexpectedResponse(msg_header.message_id));
        }

        if msg_header.token != Self::TOKEN {
            return Err(Error::UnexpectedToken(msg_header.token));
        }

        // Safety: Self::new guarantees that the channel is owned by the caller.
        let response = unsafe { self.memory.payload::<ResponseWithStatus<C::Response>>() };

        let status = response.status()?;
        if status != StatusCode::Standard(StandardStatusCode::Success) {
            return Err(Error::Status(status));
        }

        Ok(response.payload)
    }

    /// Checks if there was a channel error.
    pub fn has_error(&self) -> bool {
        self.memory.channel_status().contains(ChannelStatus::ERROR)
    }

    /// Clears channel error.
    pub fn clear_error(&mut self) {
        // Safety: Self::new guarantees that the channel is owned by the caller.
        unsafe {
            self.memory
                .set_channel_status(self.memory.channel_status() - ChannelStatus::ERROR);
        }
    }
}

/// Doorbell interface.
pub trait Doorbell {
    /// Rings the doorbell.
    fn ring(&mut self);
}

/// Shared memory and doorbell based transport implementation.
pub struct SharedMemoryTransport<D: Doorbell> {
    memory: SharedMemory,
    doorbell: D,
}

impl<D: Doorbell> SharedMemoryTransport<D> {
    /// Creates new instance.
    pub fn new(memory: SharedMemory, doorbell: D) -> Self {
        Self { memory, doorbell }
    }

    /// Acquires a new owned channel by waiting until becomes free.
    fn acquire_channel(&mut self) -> OwnedChannel<'_> {
        OwnedChannel::new(&mut self.memory)
    }
}

impl<D: Doorbell> Transport for SharedMemoryTransport<D> {
    fn invoke_command<C: Command>(&mut self, command: C) -> Result<C::Response, Error> {
        self.acquire_channel().write_command(command)?;

        self.doorbell.ring();

        // Busy-waiting until the channel becomes free.
        let mut channel = self.acquire_channel();

        if channel.has_error() {
            channel.clear_error();
            return Err(Error::ChannelError);
        }

        channel.read_response::<C>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{VendorSpecificProtocolId, Version, base, system_power};

    struct HookDoorbell<'a, F: FnMut(&mut [u32])> {
        hook: F,
        buffer: &'a mut [u32],
    }

    impl<'a, F: FnMut(&mut [u32])> HookDoorbell<'a, F> {
        pub fn new(hook: F, buffer: &'a mut [u32]) -> Self {
            Self { hook, buffer }
        }
    }

    impl<'a, F: FnMut(&mut [u32])> Doorbell for HookDoorbell<'a, F> {
        fn ring(&mut self) {
            (self.hook)(self.buffer);
        }
    }

    struct Harness {
        buffer: [u32; Self::WORD_COUNT],
    }

    impl Harness {
        const WORD_COUNT: usize = 16;
        pub const CHANNEL_STATUS_OFFSET: usize = 1;
        pub const CHANNEL_FLAGS_OFFSET: usize = 4;
        pub const LENGTH_OFFSET: usize = 5;
        pub const MSG_HEADER_OFFSET: usize = 6;
        pub const MSG_PAYLOAD_OFFSET: usize = 7;

        /// Create new instance.
        pub fn new() -> Self {
            Self {
                buffer: [0; Self::WORD_COUNT],
            }
        }

        /// Creates shared memory and doorbell based transport.
        fn create_transport<F: FnMut(&mut [u32])>(
            &mut self,
            hook: F,
        ) -> SharedMemoryTransport<HookDoorbell<'_, F>> {
            // Safety: The pointer is valid and points to self.buffer. buffer is also passed to
            // HookDoorbell, but it is only accessed when the transport layer rings the doorbell.
            // There is no concurrent accesses to the memory.
            let memory = unsafe {
                SharedMemory::new(
                    NonNull::new(self.buffer.as_mut_ptr()).unwrap(),
                    Self::WORD_COUNT * 4,
                )
            };

            SharedMemoryTransport::new(memory, HookDoorbell::new(hook, &mut self.buffer))
        }
    }

    #[test]
    fn header_size() {
        assert_eq!(0x1c, size_of::<SharedMemoryHeader>());
    }

    #[test]
    fn invoke_command() {
        let mut harness = Harness::new();

        let mut transport = harness.create_transport(|buffer| {
            assert_eq!(0, buffer[Harness::CHANNEL_STATUS_OFFSET]);
            assert_eq!(0, buffer[Harness::CHANNEL_FLAGS_OFFSET]);
            assert_eq!(4, buffer[Harness::LENGTH_OFFSET]);
            assert_eq!((0x10 << 10), buffer[Harness::MSG_HEADER_OFFSET]);

            buffer[Harness::CHANNEL_STATUS_OFFSET] = 0x1;
            buffer[Harness::LENGTH_OFFSET] = 0xc;
            buffer[Harness::MSG_PAYLOAD_OFFSET] = 0;
            buffer[Harness::MSG_PAYLOAD_OFFSET + 1] = 0x1234_5678;
        });
        let version = transport.invoke_command(base::ProtocolVersion {}).unwrap();
        assert_eq!(Version::new(0x1234, 0x5678), version.version);
    }

    #[test]
    fn command_too_long() {
        let mut harness = Harness::new();

        #[derive(Clone, Debug, PartialEq, Eq, FromBytes, IntoBytes, Immutable)]
        struct LongCommand {
            buffer: [u32; 32],
        }

        impl Command for LongCommand {
            const ID: MessageId =
                MessageId::VendorSpecific(VendorSpecificProtocolId::new(0xfc), 0xba);

            type Response = ();
        }

        let mut transport = harness.create_transport(|_buffer| {});
        assert_eq!(
            Err(Error::PayloadExceedsMaxSize),
            transport.invoke_command(LongCommand { buffer: [0; 32] })
        );
    }

    #[test]
    fn response_too_long() {
        let mut harness = Harness::new();

        let mut transport = harness.create_transport(|buffer| {
            buffer[Harness::CHANNEL_STATUS_OFFSET] = 0x1;
            buffer[Harness::LENGTH_OFFSET] = 0xffff_ffff;
        });
        assert_eq!(
            Err(Error::PayloadExceedsMaxSize),
            transport.invoke_command(base::ProtocolVersion {})
        );
    }

    #[test]
    fn invalid_response_id() {
        let mut harness = Harness::new();

        let mut transport = harness.create_transport(|buffer| {
            buffer[Harness::CHANNEL_STATUS_OFFSET] = 0x1;
            buffer[Harness::LENGTH_OFFSET] = 0xc;
            buffer[Harness::MSG_HEADER_OFFSET] = 0x12 << 10; // Unexpected ID
        });

        assert_eq!(
            Err(Error::UnexpectedResponse(MessageId::SystemPowerManagement(
                system_power::SystemPowerCommandMessageId::ProtocolVersion
            ))),
            transport.invoke_command(base::ProtocolVersion {})
        );
    }

    #[test]
    fn invalid_token() {
        let mut harness = Harness::new();

        let mut transport = harness.create_transport(|buffer| {
            buffer[Harness::CHANNEL_STATUS_OFFSET] = 0x1;
            buffer[Harness::LENGTH_OFFSET] = 0xc;
            buffer[Harness::MSG_HEADER_OFFSET] |= 0xab << 18; // Set token
        });
        assert_eq!(
            Err(Error::UnexpectedToken(0xab)),
            transport.invoke_command(base::ProtocolVersion {})
        );
    }

    #[test]
    fn invalid_response_header() {
        let mut harness = Harness::new();

        let mut transport = harness.create_transport(|buffer| {
            buffer[Harness::CHANNEL_STATUS_OFFSET] = 0x1;
            buffer[Harness::LENGTH_OFFSET] = 0xc;
            buffer[Harness::MSG_HEADER_OFFSET] = (0x11 << 10) | (1 << 8);
        });
        assert_eq!(
            Err(Error::InvalidMessageType(1)),
            transport.invoke_command(base::ProtocolVersion {})
        );
    }

    #[test]
    fn status_error() {
        let mut harness = Harness::new();

        let mut transport = harness.create_transport(|buffer| {
            buffer[Harness::CHANNEL_STATUS_OFFSET] = 0x1;
            buffer[Harness::LENGTH_OFFSET] = 0xc;
            buffer[Harness::MSG_PAYLOAD_OFFSET] = 0xffff_ffff;
            buffer[Harness::MSG_PAYLOAD_OFFSET + 1] = 0x1234_5678;
        });
        assert_eq!(
            Err(Error::Status(StatusCode::Standard(
                StandardStatusCode::NotSupported
            ))),
            transport.invoke_command(base::ProtocolVersion {})
        );
    }

    #[test]
    fn channel_error() {
        let mut harness = Harness::new();

        let mut transport = harness.create_transport(|buffer| {
            buffer[Harness::CHANNEL_STATUS_OFFSET] = 0x3;
            buffer[Harness::LENGTH_OFFSET] = 0xc;
            buffer[Harness::MSG_PAYLOAD_OFFSET] = 0;
            buffer[Harness::MSG_PAYLOAD_OFFSET + 1] = 0x1234_5678;
        });
        assert_eq!(
            Err(Error::ChannelError),
            transport.invoke_command(base::ProtocolVersion {})
        );
    }
}
