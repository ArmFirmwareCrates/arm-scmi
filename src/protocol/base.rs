// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::{
    Error,
    protocol::{MessageId, NotifyEnable, ProtocolId, define_command, define_protocol},
};

define_protocol!(
    Base,
    BaseCommandMessageId {
        DiscoverVendor = 0x3,
        DiscoverSubVendor = 0x4,
        DiscoverImplementationVersion = 0x5,
        DiscoverListProtocol = 0x6,
        DiscoverAgent = 0x7,
        NotifyErrors = 0x8,
        SetDevicePermission = 0x9,
        SetProtocolPermissions = 0xa,
        ResetAgentConfiguration = 0xb,
    }
);

define_command!(
    "BASE_DISCOVER_VENDOR",
    DiscoverVendor,
    MessageId::Base(BaseCommandMessageId::DiscoverVendor),
    {},
    { vendor_identifier: [u8; 16] }
);

define_command!(
    "BASE_DISCOVER_SUB_VENDOR",
    DiscoverSubVendor,
    MessageId::Base(BaseCommandMessageId::DiscoverSubVendor),
    {},
    { vendor_identifier: [u8; 16] }
);

define_command!(
    "BASE_DISCOVER_IMPLEMENTATION_VERSION",
    DiscoverImplementationVersion,
    MessageId::Base(BaseCommandMessageId::DiscoverImplementationVersion),
    {},
    { implementation_version: u32 }
);

define_command!(
    "BASE_DISCOVER_LIST_PROTOCOLS",
    DiscoverListProtocol,
    MessageId::Base(BaseCommandMessageId::DiscoverListProtocol),
    { skip: u32 },
    { num_protocols: u32, protocols: [u32; 4] } // TODO: this should be generic
);

define_command!(
    "BASE_DISCOVER_AGENT",
    DiscoverAgent,
    MessageId::Base(BaseCommandMessageId::DiscoverAgent),
    { agent_id: u32 },
    {
        agent_id: u32,
        name: [u8; 16],
    }
);

define_command!(
    "BASE_NOTIFY_ERRORS",
    NotifyErrors,
    MessageId::Base(BaseCommandMessageId::NotifyErrors),
    { notify_enable: NotifyEnable },
    { }
);

define_command!(
    "BASE_SET_DEVICE_PERMISSIONS",
    SetDevicePermission,
    MessageId::Base(BaseCommandMessageId::SetDevicePermission),
    {
        agent_id: u32,
        device_id: u32,
        flags: SetPermissionFlags,
    },
    { }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct SetPermissionFlags(u32);
bitflags! {
    impl SetPermissionFlags: u32 {
        const ALLOW = 1 << 0;
    }
}

define_command!(
    "BASE_SET_PROTOCOL_PERMISSIONS",
    SetProtocolPermissions,
    MessageId::Base(BaseCommandMessageId::SetProtocolPermissions),
    {
        agent_id: u32,
        device_id: u32,
        command_id: SetProtocolPermissionsCommand,
        flags: SetPermissionFlags,
    },
    { }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct SetProtocolPermissionsCommand(u32);

impl SetProtocolPermissionsCommand {
    pub fn new(protocol: ProtocolId) -> Self {
        Self(u8::from(protocol) as u32)
    }

    pub fn protocol_id(&self) -> Result<ProtocolId, Error> {
        (self.0 as u8).try_into()
    }
}

define_command!(
    "BASE_RESET_AGENT_CONFIGURATION",
    ResetAgentConfiguration,
    MessageId::Base(BaseCommandMessageId::ResetAgentConfiguration),
    {
        agent_id: u32,
        flags: ResetAgentConfigurationFlags,
    },
    { }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct ResetAgentConfigurationFlags(u32);
bitflags! {
    impl ResetAgentConfigurationFlags: u32 {
        const CLEAR = 1 << 0;
    }
}
