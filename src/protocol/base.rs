// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::protocol::{MessageId, define_command, define_protocol};

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
    { notify_enable: u32 },
    { }
);

define_command!(
    "BASE_SET_DEVICE_PERMISSIONS",
    SetDevicePermission,
    MessageId::Base(BaseCommandMessageId::SetDevicePermission),
    {
        agent_id: u32,
        device_id: u32,
        flags: u32,
    },
    { }
);

define_command!(
    "BASE_SET_PROTOCOL_PERMISSIONS",
    SetProtocolPermissions,
    MessageId::Base(BaseCommandMessageId::SetProtocolPermissions),
    {
        agent_id: u32,
        device_id: u32,
        command_id: u32, // TODO:
        flags: u32,
    },
    { }
);

define_command!(
    "BASE_RESET_AGENT_CONFIGURATION",
    ResetAgentConfiguration,
    MessageId::Base(BaseCommandMessageId::ResetAgentConfiguration),
    {
        agent_id: u32,
        flags: u32,
    },
    { }
);
