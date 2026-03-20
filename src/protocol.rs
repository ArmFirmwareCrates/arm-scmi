// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Base protocol implementation.
pub mod base;
/// Power Domain Management protocol implementation.
pub mod power_domain;
/// System Power Management protocol implementation.
pub mod system_power;

use crate::{
    Error,
    protocol::{
        base::BaseCommandMessageId, power_domain::PowerDomainCommandMessageId,
        system_power::SystemPowerCommandMessageId,
    },
};
use bitflags::bitflags;
use core::fmt::Debug;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Standard protocol identifiers.
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

    /// Creates new instance
    pub const fn new(id: u8) -> Self {
        assert!(Self::is_vendor_specific(id));
        Self(id)
    }

    /// Returns true if the protocol ID number belongs to a vendor specific protocol.
    pub const fn is_vendor_specific(value: u8) -> bool {
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
        Ok(if let Ok(id) = VendorSpecificProtocolId::try_from(value) {
            Self::VendorSpecific(id)
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

/// Message header structure.
///
/// See Table 3: Message header format
#[derive(Clone, Debug, PartialEq, Eq)]
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
    const MESSAGE_TYPE: u32 = 0x0;

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

        (token << MessageHeader::TOKEN_SHIFT)
            | (u32::from(protocol_id) << MessageHeader::PROTOCOL_ID_SHIFT)
            | (MessageHeader::MESSAGE_TYPE << MessageHeader::MESSAGE_TYPE_SHIFT)
            | (u32::from(message_id) << MessageHeader::MESSAGE_ID_SHIFT)
    }
}

/// Response structure including the status code.
#[repr(C, align(4))]
pub struct ResponseWithStatus<R> {
    status: i32,
    pub payload: R,
}

impl<R> ResponseWithStatus<R> {
    pub fn status(&self) -> Result<StatusCode, Error> {
        self.status.try_into()
    }
}

/// Command trait for connecting command structures with their message IDs and response types.
pub trait Command: Debug + FromBytes + IntoBytes + Immutable {
    const ID: MessageId;
    type Response: Debug + FromBytes + IntoBytes + Immutable;
}

/// The macro defines the common components on an SCMI protocol implementation. This includes
/// * Message ID enum type, that includes the mandatory message IDs (PROTOCOL_VERSION,
///   PROTOCOL_ATTRIBUTES, etc.)
/// * Matching command definitions for these mandatory messages.
///
/// Arguments:
/// * protocol: Identifier of the protocol.
/// * attribute_type: Type of the `attributes` field of the `PROTOCOL_ATTRIBUTES` response.
/// * name: Name of the message ID enum.
/// * variant, disc: List of enum variants and their discriminator (i.e. the raw message ID).
#[macro_export]
macro_rules! define_protocol {
    ($protocol:ident, $attribute_type:ty, $name:ident { $($variant:ident $(= $disc:expr)?),* $(,)?}) => {
        #[derive(Clone, Copy, Debug, Eq, num_enum::IntoPrimitive, PartialEq, num_enum::TryFromPrimitive)]
        #[num_enum(error_type(name = $crate::protocol::Error, constructor = $crate::protocol::Error::InvalidMessageId))]
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
            $crate::protocol::MessageId::$protocol($name::ProtocolVersion),
            {},
            { version: $crate::protocol::Version }
        );

        $crate::protocol::define_command!(
            "NEGOTIATE_PROTOCOL_VERSION",
            NegotiateProtocolVersion,
            $crate::protocol::MessageId::$protocol($name::NegotiateProtocolVersion),
            { version: $crate::protocol::Version },
            {}
        );

        $crate::protocol::define_command!(
            "PROTOCOL_ATTRIBUTES",
            ProtocolAttributes,
            $crate::protocol::MessageId::$protocol($name::ProtocolAttributes),
            {},
            { attributes: $attribute_type }
        );

        $crate::protocol::define_command!(
            "PROTOCOL_MESSAGE_ATTRIBUTES",
            ProtocolMessageAttributes,
            $crate::protocol::MessageId::$protocol($name::ProtocolMessageAttributes),
            { message_id: u32 },
            { attributes: u32 }
        );
    };
}

pub use define_protocol;

/// Defines SCMI command and response structures.
///
/// The macro defines two structs for the command and its response. It also implements the `Command`
/// trait for the defined command structure.
///
/// Arguments:
/// * name: Name of the command as written in the specification
/// * command: Name of the command structure. The response structure's name will be automatically
///   generated as \[command + 'Response'\].
/// * msg_id: [MessageId] of the command.
/// * c_field, c_type: Command fields and their types.
/// * r_field, r_type: Response fields and their types.
#[macro_export]
macro_rules! define_command {
    ($name:literal, $command:ident, $msg_id:expr, { $($c_field:ident: $c_type:ty),* $(,)?}, { $($r_field:ident: $r_type:ty),* $(,)?}) => {
        #[doc = "`"]
        #[doc = $name]
        #[doc = "` command."]
        #[derive(Clone, Debug, PartialEq, Eq, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::IntoBytes, zerocopy::KnownLayout)]
        #[repr(C, align(4))]
        pub struct $command {
            $(pub $c_field: $c_type),*
        }

        impl $crate::protocol::Command for $command {
            const ID: $crate::protocol::MessageId = $msg_id;
            type Response = paste::paste!{ [<$command Response>] };
        }

        paste::paste! {
            #[doc = "`"]
            #[doc = $name]
            #[doc = "` command response."]
            #[derive(Clone, Debug, PartialEq, Eq, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::IntoBytes, zerocopy::KnownLayout)]
            #[repr(C, align(4))]
            pub struct [<$command Response>] {
                $(pub $r_field: $r_type),*
            }
        }
    };
}

pub use define_command;

/// Common message ID type for storing the message ID of all protocols. Each protocol must have
/// its own variant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageId {
    Base(BaseCommandMessageId),
    PowerDomainManagement(PowerDomainCommandMessageId),
    SystemPowerManagement(SystemPowerCommandMessageId),
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
                _ => return Err(Error::ProtocolNotSupported),
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

fn get_ascii_string(slice: &[u8]) -> Option<&str> {
    let end_of_string = slice.iter().position(|c| *c == 0)?;
    core::str::from_utf8(&slice[..end_of_string]).ok()
}

/// Protocol version type.
///
/// Commonly used in `*_PROTOCOL_VERSION` and `*_NEGOTIATE_PROTOCOL_VERSION` commands.
#[derive(Clone, Debug, Default, PartialEq, Eq, FromBytes, Immutable, IntoBytes, KnownLayout)]
#[repr(transparent)]
pub struct Version(u32);

impl Version {
    const MAJOR_MASK: u32 = 0xffff;
    const MAJOR_SHIFT: u32 = 16;

    const MINOR_MASK: u32 = 0xffff;
    const MINOR_SHIFT: u32 = 0;

    /// Creates new instance.
    pub const fn new(major: u16, minor: u16) -> Self {
        Self((major as u32) << Self::MAJOR_SHIFT | (minor as u32) << Self::MINOR_SHIFT)
    }

    /// Returns major version.
    pub const fn major(&self) -> u16 {
        ((self.0 >> Self::MAJOR_SHIFT) & Self::MAJOR_MASK) as u16
    }

    /// Returns minor version.
    pub const fn minor(&self) -> u16 {
        ((self.0 >> Self::MINOR_SHIFT) & Self::MINOR_MASK) as u16
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
    const VENDOR_SPECIFIC_BASE: i32 = -128;
}

impl TryFrom<i32> for StatusCode {
    type Error = Error;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        Ok(if value <= Self::VENDOR_SPECIFIC_BASE {
            Self::VendorSpecific(value)
        } else {
            StatusCode::Standard(StandardStatusCode::try_from_primitive(value)?)
        })
    }
}

impl From<StatusCode> for i32 {
    fn from(value: StatusCode) -> Self {
        match value {
            StatusCode::Standard(code) => code.into(),
            StatusCode::VendorSpecific(code) => {
                assert!(code <= StatusCode::VENDOR_SPECIFIC_BASE);
                code
            }
        }
    }
}

/// Bitfield for enabling notifications.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct NotifyEnable(u32);
bitflags! {
    impl NotifyEnable: u32 {
        /// Enables sending notification messages to the calling agent.
        const ENABLE = 1 << 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zerocopy::transmute_ref;

    #[test]
    fn protocol_id() {
        assert_eq!(
            Ok(ProtocolId::Standard(StandardProtocolId::Base)),
            ProtocolId::try_from(0x10)
        );

        assert_eq!(
            Ok(ProtocolId::Standard(StandardProtocolId::SystemTelemetry)),
            ProtocolId::try_from(0x1b)
        );

        assert_eq!(
            Ok(ProtocolId::VendorSpecific(VendorSpecificProtocolId::new(
                0x80
            ))),
            ProtocolId::try_from(0x80)
        );

        assert!(ProtocolId::try_from(0x0f).is_err());

        assert!(ProtocolId::try_from(0x1c).is_err());

        assert_eq!(
            0x10_u8,
            ProtocolId::Standard(StandardProtocolId::Base).into()
        );

        assert_eq!(
            0x15_u8,
            ProtocolId::Standard(StandardProtocolId::SensorManagement).into()
        );

        assert_eq!(
            0xc0_u8,
            ProtocolId::VendorSpecific(VendorSpecificProtocolId::new(0xc0)).into()
        );

        assert!(VendorSpecificProtocolId::try_from(0x10).is_err());
    }

    #[test]
    #[should_panic]
    fn protocol_id_invalid() {
        VendorSpecificProtocolId::new(0x1);
    }

    #[test]
    fn message_header() {
        let base_protocol_version_bits: u32 = (0x3ff << 18) | (0x10 << 10);
        let header = MessageHeader::try_from(base_protocol_version_bits).unwrap();
        assert_eq!(
            MessageId::Base(BaseCommandMessageId::ProtocolVersion),
            header.message_id
        );
        assert_eq!(0x3ff, header.token);

        let invalid_protocol: u32 = (0x3ff << 18) | (0x0f << 10);
        assert!(MessageHeader::try_from(invalid_protocol).is_err());

        let invalid_message_type: u32 = (0x3ff << 18) | (0x10 << 10) | (0b01 << 8);
        assert!(MessageHeader::try_from(invalid_message_type).is_err());

        let invalid_message_id: u32 = (0x3ff << 18) | (0x10 << 10) | 0xff;
        assert!(MessageHeader::try_from(invalid_message_id).is_err());

        let vendor_specific_bits: u32 = (0x3ff << 18) | (0xa0 << 10) | 0xb0;
        let header = MessageHeader::try_from(vendor_specific_bits).unwrap();
        assert_eq!(
            MessageId::VendorSpecific(VendorSpecificProtocolId::new(0xa0), 0xb0),
            header.message_id
        );
        assert_eq!(0x3ff, header.token);

        let header = MessageHeader {
            token: 0x3ff,
            message_id: MessageId::Base(BaseCommandMessageId::DiscoverAgent),
        };
        let power_domain_name_get_bits: u32 = (0x3ff << 18) | (0x10 << 10) | 0x07;
        assert_eq!(power_domain_name_get_bits, header.into());

        let header = MessageHeader {
            token: 0x3ff,
            message_id: MessageId::VendorSpecific(VendorSpecificProtocolId::new(0xc2), 0xd1),
        };
        let vendor_specific_bits: u32 = (0x3ff << 18) | (0xc2 << 10) | 0xd1;
        assert_eq!(vendor_specific_bits, header.into());
    }

    #[test]
    #[should_panic]
    fn message_header_invalid_token() {
        let header = MessageHeader {
            token: 0x400,
            message_id: MessageId::Base(BaseCommandMessageId::ProtocolVersion),
        };
        let _ = u32::from(header);
    }

    #[test]
    fn response_with_status() {
        let response = ResponseWithStatus {
            status: -1,
            payload: 0,
        };

        assert_eq!(
            response.status(),
            Ok(StatusCode::Standard(StandardStatusCode::NotSupported))
        );
    }

    #[test]
    fn message_id() {
        // Base
        assert_eq!(
            (ProtocolId::Standard(StandardProtocolId::Base), 0x01),
            MessageId::Base(BaseCommandMessageId::ProtocolAttributes).into()
        );

        assert_eq!(
            Ok(MessageId::Base(BaseCommandMessageId::ProtocolAttributes)),
            (ProtocolId::Standard(StandardProtocolId::Base), 0x01).try_into()
        );

        assert!(
            MessageId::try_from((ProtocolId::Standard(StandardProtocolId::Base), 0xff)).is_err()
        );

        // PowerDomainManagement
        assert_eq!(
            (
                ProtocolId::Standard(StandardProtocolId::PowerDomainManagement),
                0x01
            ),
            MessageId::PowerDomainManagement(PowerDomainCommandMessageId::ProtocolAttributes)
                .into()
        );

        assert_eq!(
            Ok(MessageId::PowerDomainManagement(
                PowerDomainCommandMessageId::ProtocolAttributes
            )),
            (
                ProtocolId::Standard(StandardProtocolId::PowerDomainManagement),
                0x01
            )
                .try_into()
        );

        assert!(
            MessageId::try_from((
                ProtocolId::Standard(StandardProtocolId::PowerDomainManagement),
                0xff
            ))
            .is_err()
        );

        // SystemPowerManagement
        assert_eq!(
            (
                ProtocolId::Standard(StandardProtocolId::SystemPowerManagement),
                0x01
            ),
            MessageId::SystemPowerManagement(SystemPowerCommandMessageId::ProtocolAttributes)
                .into()
        );

        assert_eq!(
            Ok(MessageId::SystemPowerManagement(
                SystemPowerCommandMessageId::ProtocolAttributes
            )),
            (
                ProtocolId::Standard(StandardProtocolId::SystemPowerManagement),
                0x01
            )
                .try_into()
        );

        assert!(
            MessageId::try_from((
                ProtocolId::Standard(StandardProtocolId::SystemPowerManagement),
                0xff
            ))
            .is_err()
        );

        // Vendor Specific
        assert_eq!(
            (
                ProtocolId::VendorSpecific(VendorSpecificProtocolId::new(0x90)),
                0x96
            ),
            MessageId::VendorSpecific(VendorSpecificProtocolId::new(0x90), 0x96).into()
        );

        assert_eq!(
            Ok(MessageId::VendorSpecific(
                VendorSpecificProtocolId::new(0x90),
                0x96
            )),
            (
                ProtocolId::VendorSpecific(VendorSpecificProtocolId::new(0x90)),
                0x96
            )
                .try_into()
        );

        // Recognized, but not supported
        assert!(
            MessageId::try_from((
                ProtocolId::Standard(StandardProtocolId::PerformanceDomainManagement),
                0x0
            ))
            .is_err()
        );
    }

    #[test]
    fn version() {
        let version = Version::new(0xabcd, 0x8901);

        assert_eq!(0xabcd, version.major());
        assert_eq!(0x8901, version.minor());

        let version_raw: &u32 = transmute_ref!(&version);
        assert_eq!(0xabcd8901_u32, *version_raw);
    }

    #[test]
    fn status_codes() {
        assert_eq!(
            0_i32,
            StatusCode::Standard(StandardStatusCode::Success).into()
        );

        assert_eq!(
            Ok(StatusCode::Standard(StandardStatusCode::Success)),
            0.try_into()
        );

        assert_eq!(
            -12_i32,
            StatusCode::Standard(StandardStatusCode::PartialError).into()
        );

        assert_eq!(
            Ok(StatusCode::Standard(StandardStatusCode::PartialError)),
            (-12).try_into()
        );

        assert_eq!(-128_i32, StatusCode::VendorSpecific(-128).into());

        assert_eq!(Ok(StatusCode::VendorSpecific(-128)), (-128).try_into());
    }

    #[test]
    #[should_panic]
    fn status_code_invalid() {
        let _ = i32::from(StatusCode::VendorSpecific(-1));
    }
}
