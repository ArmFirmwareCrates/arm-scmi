// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Error,
    protocol::{
        MessageId, NotifyEnable, ProtocolId, StandardProtocolId, define_command, define_protocol,
        get_ascii_string,
    },
};
use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, transmute_ref};

/// Base protocol attributes.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, Immutable, IntoBytes, KnownLayout,
)]
#[repr(transparent)]
pub struct BaseProtocolAttributes(u32);

impl BaseProtocolAttributes {
    const AGENTS_MASK: u32 = 0xff;
    const AGENTS_SHIFT: u32 = 8;

    const PROTOCOLS_MASK: u32 = 0xff;
    const PROTOCOLS_SHIFT: u32 = 0;

    /// Sets the number of agents in the system.
    ///
    /// There cannot be more than 255 agents.
    pub const fn set_agent_count(&mut self, count: usize) {
        assert!(count as u32 & !Self::AGENTS_MASK == 0);

        self.0 &= !(Self::AGENTS_MASK << Self::AGENTS_SHIFT);
        self.0 |= ((count as u32) & Self::AGENTS_MASK) << Self::AGENTS_SHIFT;
    }

    /// Returns the number of agents in the system.
    pub const fn agent_count(&self) -> usize {
        ((self.0 >> Self::AGENTS_SHIFT) & Self::AGENTS_MASK) as usize
    }

    /// Sets the number of protocols that are implemented, excluding the Base protocol.
    ///
    /// There cannot be more than 256 protocols, including the Base protocol.
    pub const fn set_protocol_count(&mut self, count: usize) {
        assert!(count as u32 & !Self::PROTOCOLS_MASK == 0);

        self.0 &= !(Self::PROTOCOLS_MASK << Self::PROTOCOLS_SHIFT);
        self.0 |= ((count as u32) & Self::PROTOCOLS_MASK) << Self::PROTOCOLS_SHIFT;
    }

    /// Returns the number of protocols that are implemented, excluding the Base protocol.
    pub const fn protocol_count(&self) -> usize {
        ((self.0 >> Self::PROTOCOLS_SHIFT) & Self::PROTOCOLS_MASK) as usize
    }
}

/// `BASE_SET_DEVICE_PERMISSIONS` and `BASE_SET_PROTOCOL_PERMISSIONS` flags.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct BasePermissionFlags(u32);

bitflags! {
    impl BasePermissionFlags: u32 {
        /// Allows agent access to the protocol/device.
        const ACCESS_TYPE = 1 << 0;
    }
}

/// `BASE_SET_PROTOCOL_PERMISSIONS` command ID.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct BaseSetProtocolPermissionsCommand(u32);

impl BaseSetProtocolPermissionsCommand {
    /// Creates new instance.
    ///
    /// Using Base protocol as the `protocol` value is not permitted here. It is mandatory to
    /// implement the Base protocol for all agents.
    pub fn new(protocol: ProtocolId) -> Self {
        assert_ne!(protocol, ProtocolId::Standard(StandardProtocolId::Base));

        Self(u8::from(protocol) as u32)
    }

    /// Returns protocol ID.
    pub fn protocol_id(&self) -> Result<ProtocolId, Error> {
        (self.0 as u8).try_into()
    }
}

impl From<ProtocolId> for BaseSetProtocolPermissionsCommand {
    fn from(value: ProtocolId) -> Self {
        Self::new(value)
    }
}

/// `BASE_RESET_AGENT_CONFIGURATION` flags.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct BaseResetAgentConfigurationFlags(u32);

bitflags! {
    impl BaseResetAgentConfigurationFlags: u32 {
        /// Reset all access permission settings of the agent.
        const PERMISSIONS_RESET = 1 << 0;
    }
}

define_protocol!(
    Base,
    BaseProtocolAttributes,
    BaseCommandMessageId {
        DiscoverVendor = 0x3,
        DiscoverSubVendor = 0x4,
        DiscoverImplementationVersion = 0x5,
        DiscoverListProtocols = 0x6,
        DiscoverAgent = 0x7,
        NotifyErrors = 0x8,
        SetDevicePermissions = 0x9,
        SetProtocolPermissions = 0xa,
        ResetAgentConfiguration = 0xb,
    }
);

define_command!(
    "BASE_DISCOVER_VENDOR",
    BaseDiscoverVendor,
    MessageId::Base(BaseCommandMessageId::DiscoverVendor),
    {},
    { vendor_identifier: [u8; 16] }
);

impl BaseDiscoverVendorResponse {
    /// Returns the vendor identifier string.
    pub fn vendor_identifier(&self) -> Option<&str> {
        get_ascii_string(&self.vendor_identifier)
    }
}

define_command!(
    "BASE_DISCOVER_SUB_VENDOR",
    BaseDiscoverSubVendor,
    MessageId::Base(BaseCommandMessageId::DiscoverSubVendor),
    {},
    { vendor_identifier: [u8; 16] }
);

impl BaseDiscoverSubVendorResponse {
    /// Returns the sub vendor identifier string.
    pub fn vendor_identifier(&self) -> Option<&str> {
        get_ascii_string(&self.vendor_identifier)
    }
}

define_command!(
    "BASE_DISCOVER_IMPLEMENTATION_VERSION",
    BaseDiscoverImplementationVersion,
    MessageId::Base(BaseCommandMessageId::DiscoverImplementationVersion),
    {},
    { implementation_version: u32 }
);

define_command!(
    "BASE_DISCOVER_LIST_PROTOCOLS",
    BaseDiscoverListProtocol,
    MessageId::Base(BaseCommandMessageId::DiscoverListProtocols),
    { skip: u32 },
    { num_protocols: u32, protocols: [u32; Self::MAX_PROTOCOL_WORDS] }
);

impl BaseDiscoverListProtocolResponse {
    /// This is an arbitrary limitation to keep the type reasonably small compared to its maximal
    /// 260 byte size. Currently there are 12 standard protocols and by having this value as 4, it
    /// leaves room for up to 5 vendor specific protocols.
    const MAX_PROTOCOL_WORDS: usize = 4;

    /// Maximal number of protocols that can be stored in a single response structure.
    pub const MAX_PROTOCOL_COUNT: usize = Self::MAX_PROTOCOL_WORDS * 4;

    /// Returns an iterator to the Protocol IDs stored in the structure.
    pub fn iter(&self) -> impl Iterator<Item = Result<ProtocolId, Error>> {
        let protocol_bytes: &[u8] = transmute_ref!(&self.protocols);
        protocol_bytes[..self.num_protocols as usize]
            .iter()
            .map(|b| ProtocolId::try_from(*b))
    }

    /// Returns true if a protocol is included in the list.
    pub fn contains_protocol(&self, protocol_id: ProtocolId) -> bool {
        self.iter().any(|protocol| protocol == Ok(protocol_id))
    }
}

define_command!(
    "BASE_DISCOVER_AGENT",
    BaseDiscoverAgent,
    MessageId::Base(BaseCommandMessageId::DiscoverAgent),
    { agent_id: u32 },
    {
        agent_id: u32,
        name: [u8; 16],
    }
);

impl BaseDiscoverAgentResponse {
    /// Returns the agent name.
    pub fn name(&self) -> Option<&str> {
        get_ascii_string(&self.name)
    }
}

define_command!(
    "BASE_NOTIFY_ERRORS",
    BaseNotifyErrors,
    MessageId::Base(BaseCommandMessageId::NotifyErrors),
    { notify_enable: NotifyEnable },
    { }
);

define_command!(
    "BASE_SET_DEVICE_PERMISSIONS",
    BaseSetDevicePermission,
    MessageId::Base(BaseCommandMessageId::SetDevicePermissions),
    {
        agent_id: u32,
        device_id: u32,
        flags: BasePermissionFlags,
    },
    { }
);

define_command!(
    "BASE_SET_PROTOCOL_PERMISSIONS",
    BaseSetProtocolPermissions,
    MessageId::Base(BaseCommandMessageId::SetProtocolPermissions),
    {
        agent_id: u32,
        device_id: u32,
        command_id: BaseSetProtocolPermissionsCommand,
        flags: BasePermissionFlags,
    },
    { }
);

define_command!(
    "BASE_RESET_AGENT_CONFIGURATION",
    BaseResetAgentConfiguration,
    MessageId::Base(BaseCommandMessageId::ResetAgentConfiguration),
    {
        agent_id: u32,
        flags: BaseResetAgentConfigurationFlags,
    },
    { }
);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::StandardProtocolId;

    #[test]
    fn base_protocol_attributes() {
        let mut attributes = BaseProtocolAttributes((5 << 8) | 3);

        assert_eq!(attributes.agent_count(), 5);
        assert_eq!(attributes.protocol_count(), 3);

        attributes.set_agent_count(0x10);
        attributes.set_protocol_count(0x20);

        assert_eq!(attributes.0, 0x1020);
    }

    #[test]
    fn permission_command() {
        let command = BaseSetProtocolPermissionsCommand::new(ProtocolId::Standard(
            StandardProtocolId::PowerDomainManagement,
        ));

        assert_eq!(
            command.protocol_id(),
            Ok(ProtocolId::Standard(
                StandardProtocolId::PowerDomainManagement
            ))
        );

        let command = BaseSetProtocolPermissionsCommand(0);
        assert!(command.protocol_id().is_err());

        let command = BaseSetProtocolPermissionsCommand::from(ProtocolId::Standard(
            StandardProtocolId::PowerDomainManagement,
        ));
        assert_eq!(
            command.protocol_id(),
            Ok(ProtocolId::Standard(
                StandardProtocolId::PowerDomainManagement
            ))
        );
    }

    #[test]
    #[should_panic]
    fn permission_command_base() {
        BaseSetProtocolPermissionsCommand::new(ProtocolId::Standard(StandardProtocolId::Base));
    }

    #[test]
    fn vendor_identifier() {
        let mut response = BaseDiscoverVendorResponse {
            vendor_identifier: [b'A'; 16],
        };

        response.vendor_identifier[..6].copy_from_slice(b"Hello\0");
        assert_eq!(response.vendor_identifier(), Some("Hello"));
    }

    #[test]
    fn sub_vendor_identifier() {
        let mut response = BaseDiscoverSubVendorResponse {
            vendor_identifier: [b'A'; 16],
        };

        response.vendor_identifier[..6].copy_from_slice(b"Hello\0");
        assert_eq!(response.vendor_identifier(), Some("Hello"));
    }

    #[test]
    fn protocol_iterator() {
        let response = BaseDiscoverListProtocolResponse {
            num_protocols: 4,
            protocols: {
                let mut protocols = [0u32; BaseDiscoverListProtocolResponse::MAX_PROTOCOL_WORDS];
                protocols[0] = u32::from_ne_bytes([0x10, 0x80, 0x00, 0x11]);
                protocols
            },
        };

        let mut iter = response.iter();
        assert_eq!(
            iter.next(),
            Some(Ok(ProtocolId::Standard(StandardProtocolId::Base)))
        );
        assert_eq!(iter.next(), Some(Ok(ProtocolId::try_from(0x80).unwrap())));
        assert!(iter.next().unwrap().is_err());
        assert_eq!(
            iter.next(),
            Some(Ok(ProtocolId::Standard(
                StandardProtocolId::PowerDomainManagement
            )))
        );
        assert_eq!(iter.next(), None);

        assert!(response.contains_protocol(ProtocolId::Standard(StandardProtocolId::Base)));
        assert!(!response.contains_protocol(ProtocolId::Standard(
            StandardProtocolId::SystemPowerManagement
        )));
    }

    #[test]
    fn agent_name() {
        let mut response = BaseDiscoverAgentResponse {
            agent_id: 0,
            name: [b'A'; 16],
        };

        response.name[..6].copy_from_slice(b"Hello\0");
        assert_eq!(response.name(), Some("Hello"));
    }
}
