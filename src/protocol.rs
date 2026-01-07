// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

pub mod base;
pub mod power_domain;
pub mod system_power;

use crate::{
    Error,
    protocol::{
        base::BaseCommandMessageId, power_domain::PowerDomainCommandMessageId,
        system_power::SystemPowerCommandMessageId,
    },
};
use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Standard protocol identifers.
///
/// See Table 2: Protocol identifiers
#[derive(Clone, Copy, Debug, Eq, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[num_enum(error_type(name = Error, constructor = Error::InvalidProtocolId))]
#[repr(u8)]
pub enum StandardProtocolId {
    // 0x00-0x0f: Reserved
    Base = 0x10,
    PowerDomainManagement = 0x11,
    SystemPowerManagement = 0x12,
    PerformanceDomainManagement = 0x13,
    ClockManagement = 0x14,
    SensorManagement = 0x15,
    ResetDomainManagement = 0x16,
    VoltageDomainManagement = 0x17,
    PowerCappingAndMonitoring = 0x18,
    PinControl = 0x19,
    MpamFb = 0x1a,
    SystemTelemetry = 0x1b,
    // 0x1c-0x7f: Reserved for future use by this specification
    // 0x80-0xff: Reserved for vendor or platform-specific extensions to this interface.
}

/// Vendor or platform specific protocol ID. The valid ID range is 0x80-0xff.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VendorSpecificProtocolId(u8);

impl VendorSpecificProtocolId {
    const VENDOR_SPEC_BASE: u8 = 0x80;

    pub fn is_vendor_specific(value: u8) -> bool {
        value >= Self::VENDOR_SPEC_BASE
    }
}

impl TryFrom<u8> for VendorSpecificProtocolId {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if Self::is_vendor_specific(value) {
            Ok(Self(value))
        } else {
            Err(Error::InvalidProtocolId(value))
        }
    }
}

impl From<VendorSpecificProtocolId> for u8 {
    fn from(value: VendorSpecificProtocolId) -> Self {
        assert!(value.0 >= VendorSpecificProtocolId::VENDOR_SPEC_BASE);
        value.0
    }
}

/// Protocol identifier type for storing both standard and vendor/platform specific protocol IDs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProtocolId {
    Standard(StandardProtocolId),
    VendorSpecific(VendorSpecificProtocolId),
}

impl TryFrom<u8> for ProtocolId {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Ok(if VendorSpecificProtocolId::is_vendor_specific(value) {
            Self::VendorSpecific(value.try_into()?)
        } else {
            Self::Standard(StandardProtocolId::try_from(value)?)
        })
    }
}

impl From<ProtocolId> for u8 {
    fn from(value: ProtocolId) -> Self {
        match value {
            ProtocolId::Standard(id) => id.into(),
            ProtocolId::VendorSpecific(id) => id.into(),
        }
    }
}

/// Message header structure
///
/// See Table 3: Message header format
pub struct MessageHeader {
    pub token: u16,
    pub message_id: MessageId,
}

impl MessageHeader {
    const TOKEN_MASK: u32 = 0x3ff;
    const TOKEN_SHIFT: u32 = 18;

    const PROTOCOL_ID_MASK: u32 = 0xff;
    const PROTOCOL_ID_SHIFT: u32 = 10;

    const MESSAGE_TYPE_MASK: u32 = 0x3;
    const MESSAGE_TYPE_SHIFT: u32 = 8;
    const MESSAGE_TYPE: u32 = 0x1;

    const MESSAGE_ID_MASK: u32 = 0xff;
    const MESSAGE_ID_SHIFT: u32 = 0;
}

impl TryFrom<u32> for MessageHeader {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        let token = ((value >> Self::TOKEN_SHIFT) & Self::TOKEN_MASK) as u16;

        let protocol_id = ProtocolId::try_from(
            ((value >> Self::PROTOCOL_ID_SHIFT) & Self::PROTOCOL_ID_MASK) as u8,
        )?;

        let message_type = (value >> Self::MESSAGE_TYPE_SHIFT) & Self::MESSAGE_TYPE_MASK;
        if message_type != Self::MESSAGE_TYPE {
            return Err(Error::InvalidMessageType(message_type as u8));
        }

        let message_id = MessageId::try_from((
            protocol_id,
            ((value >> Self::MESSAGE_ID_SHIFT) & Self::MESSAGE_ID_MASK) as u8,
        ))?;

        Ok(Self { token, message_id })
    }
}

impl From<MessageHeader> for u32 {
    fn from(value: MessageHeader) -> Self {
        let token = u32::from(value.token);
        assert_eq!(token & !MessageHeader::TOKEN_MASK, 0);

        let (protocol_id, message_id) = value.message_id.into();
        let protocol_id = u8::from(protocol_id);

        (token << MessageHeader::MESSAGE_TYPE_SHIFT)
            | (u32::from(protocol_id) << MessageHeader::PROTOCOL_ID_SHIFT)
            | (MessageHeader::MESSAGE_TYPE << MessageHeader::MESSAGE_TYPE_SHIFT)
            | (u32::from(message_id) << MessageHeader::MESSAGE_ID_SHIFT)
    }
}

/// Command trait for connecting command structures with their message IDs and response types.
pub trait Command {
    const ID: MessageId;
    type Response;
}

/// The macro defines the common components on an SCMI protocol implementation. This includes
/// * Message ID enum type, that includes the mandatory message IDs (PROTOCOL_VERSION,
///   PROTOCOL_ATTRIBUTES, etc.)
/// * Matching command definitions for these mandatory messages.
///
/// Arguments:
/// * protocol: Identifier of the protocol.
/// * name: Name of the message ID enum.
/// * variant, disc: List of enum variants and their discriminator (i.e. the raw message ID).
macro_rules! define_protocol {
    ($protocol:ident, $name:ident { $($variant:ident $(= $disc:expr)?),* $(,)?}) => {
        #[derive(Clone, Copy, Debug, Eq, num_enum::IntoPrimitive, PartialEq, num_enum::TryFromPrimitive)]
        #[num_enum(error_type(name = crate::protocol::Error, constructor = crate::protocol::Error::InvalidMessageId))]
        #[repr(u8)]
        pub enum $name {
            ProtocolVersion = 0x0,
            ProtocolAttributes = 0x1,
            ProtocolMessageAttributes = 0x2,
            NegotiateProtocolVersion = 0x10,
            $($variant $(= $disc)?),*
        }

        $crate::protocol::define_command!(
            "PROTOCOL_VERSION",
            ProtocolVersion,
            crate::protocol::MessageId::$protocol($name::ProtocolVersion),
            {},
            { version: u32 }
        );

        $crate::protocol::define_command!(
            "NEGOTIATE_PROTOCOL_VERSION",
            NegotiateProtocolVersion,
            crate::protocol::MessageId::$protocol($name::NegotiateProtocolVersion),
            { version: u32 },
            {}
        );

        $crate::protocol::define_command!(
            "PROTOCOL_ATTRIBUTES",
            ProtocolAttributes,
            crate::protocol::MessageId::$protocol($name::ProtocolAttributes),
            {},
            { attributes: u32 }
        );

        $crate::protocol::define_command!(
            "PROTOCOL_MESSAGE_ATTRIBUTES",
            ProtocolMessageAttributes,
            crate::protocol::MessageId::$protocol($name::ProtocolMessageAttributes),
            {},
            { attributes: u32 }
        );
    };


}

pub(crate) use define_protocol;

/// Defines SCMI command and response structures.
///
/// Arguments:
/// * name: Name of the command as written in the specification
/// * command: Name of the command structure. The response structure's name will be automatically
///   generated as [command + 'Response'].
/// * c_field, c_type: Command fields and their types.
/// * r_field, r_type: Response fields and their types.
macro_rules! define_command {
    ($name:literal, $command:ident, $msg_id:expr, { $($c_field:ident: $c_type:ty),* $(,)?}, { $($r_field:ident: $r_type:ty),* $(,)?}) => {
        #[doc = "`"]
        #[doc = $name]
        #[doc = "` command."]
        #[derive(Debug, PartialEq, Eq, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::IntoBytes, zerocopy::KnownLayout)]
        pub struct $command {
            $(pub $c_field: $c_type),*
        }

        impl crate::protocol::Command for $command {
            const ID: crate::protocol::MessageId = $msg_id;
            type Response = paste::paste!{ [<$command Response>] };
        }

        paste::paste! {
            #[doc = "`"]
            #[doc = $name]
            #[doc = "` command response."]
            #[derive(Debug, PartialEq, Eq, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::IntoBytes, zerocopy::KnownLayout)]
            pub struct [<$command Response>] {
                $(pub $r_field: $r_type),*
            }
        }
    };
}

pub(crate) use define_command;

/// Common message ID type for storing the message ID of all protocols. Each protocol must have
/// its own variant.
pub enum MessageId {
    Base(BaseCommandMessageId),
    PowerDomainManagement(PowerDomainCommandMessageId),
    SystemPowerManagement(SystemPowerCommandMessageId),
    // TODO: PerformanceDomainManagement,
    // TODO: ClockManagement,
    // TODO: SensorManagement,
    // TODO: ResetDomainManagement,
    // TODO: VoltageDomainManagement,
    // TODO: PowerCappingAndMonitoring,
    // TODO: PinControl,
    // TODO: MpamFb,
    // TODO: SystemTelemetry,
    VendorSpecific(VendorSpecificProtocolId, u8),
}

impl TryFrom<(ProtocolId, u8)> for MessageId {
    type Error = Error;

    fn try_from(value: (ProtocolId, u8)) -> Result<Self, Self::Error> {
        Ok(match value.0 {
            ProtocolId::Standard(protocol) => match protocol {
                StandardProtocolId::Base => Self::Base(BaseCommandMessageId::try_from(value.1)?),
                StandardProtocolId::PowerDomainManagement => {
                    Self::PowerDomainManagement(PowerDomainCommandMessageId::try_from(value.1)?)
                }
                StandardProtocolId::SystemPowerManagement => {
                    Self::SystemPowerManagement(SystemPowerCommandMessageId::try_from(value.1)?)
                }
                StandardProtocolId::PerformanceDomainManagement => todo!(),
                StandardProtocolId::ClockManagement => todo!(),
                StandardProtocolId::SensorManagement => todo!(),
                StandardProtocolId::ResetDomainManagement => todo!(),
                StandardProtocolId::VoltageDomainManagement => todo!(),
                StandardProtocolId::PowerCappingAndMonitoring => todo!(),
                StandardProtocolId::PinControl => todo!(),
                StandardProtocolId::MpamFb => todo!(),
                StandardProtocolId::SystemTelemetry => todo!(),
            },
            ProtocolId::VendorSpecific(protocol_id) => Self::VendorSpecific(protocol_id, value.1),
        })
    }
}

impl From<MessageId> for (ProtocolId, u8) {
    fn from(value: MessageId) -> Self {
        match value {
            MessageId::Base(message_id) => (
                ProtocolId::Standard(StandardProtocolId::Base),
                message_id.into(),
            ),
            MessageId::PowerDomainManagement(message_id) => (
                ProtocolId::Standard(StandardProtocolId::PowerDomainManagement),
                message_id.into(),
            ),
            MessageId::SystemPowerManagement(message_id) => (
                ProtocolId::Standard(StandardProtocolId::SystemPowerManagement),
                message_id.into(),
            ),
            MessageId::VendorSpecific(protocol_id, message_id) => {
                (ProtocolId::VendorSpecific(protocol_id), message_id)
            }
        }
    }
}

/// Standard status codes.
///
/// See Table 5: Status codes.
#[derive(Clone, Copy, Debug, Eq, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[num_enum(error_type(name = Error, constructor = Error::InvalidStatusCode))]
#[repr(i32)]
pub enum StandardStatusCode {
    Success = 0,
    NotSupported = -1,
    InvalidParameters = -2,
    Denied = -3,
    NotFound = -4,
    OutOfRange = -5,
    Busy = -6,
    CommsError = -7,
    GenericError = -8,
    HardwareError = -9,
    ProtocolError = -10,
    InUse = -11,
    PartialError = -12,
    // -13 to -127: Reserved
    // -127<: Vendor specific
}

/// Status code type for storing both standard and vendor/platform specific status codes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StatusCode {
    Standard(StandardStatusCode),
    VendorSpecific(i32),
}

impl StatusCode {
    const VENDOR_SPECIFIC: i32 = -127;
}

impl TryFrom<i32> for StatusCode {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(if value < Self::VENDOR_SPECIFIC {
            Self::VendorSpecific(value)
        } else {
            StatusCode::try_from(value)?
        })
    }
}

impl From<StatusCode> for i32 {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::Standard(code) => code.into(),
            StatusCode::VendorSpecific(code) => {
                assert!(code < StatusCode::VENDOR_SPECIFIC);
                code
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct NotifyEnable(u32);
bitflags! {
    impl NotifyEnable: u32 {
        const ENABLE = 1 << 0;
    }
}
