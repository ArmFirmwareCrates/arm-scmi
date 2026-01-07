// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

use crate::protocol::{MessageId, NotifyEnable, define_command, define_protocol};

define_protocol!(
    SystemPowerManagement,
    SystemPowerCommandMessageId {
        StateSet = 0x3,
        StateGet = 0x4,
        StateNotify = 0x5,
    }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
pub struct SystemState(u32);

impl SystemState {
    const VENDOR_SPEC_BASE: u32 = 0x80000000;
    fn is_vendor_specific(value: u32) -> bool {
        value > Self::VENDOR_SPEC_BASE
    }

    pub const SHUTDOWN: SystemState = SystemState(0x0);
    pub const COLD_RESET: SystemState = SystemState(0x1);
    pub const WARM_RESET: SystemState = SystemState(0x2);
    pub const POWER_UP: SystemState = SystemState(0x3);
    pub const SUSPEND: SystemState = SystemState(0x4);

    pub fn new_vendor_specific(value: u32) -> Self {
        assert!(Self::is_vendor_specific(value));
        Self(value)
    }
}

define_command!(
    "SYSTEM_POWER_STATE_SET",
    StateSet,
    MessageId::SystemPowerManagement(SystemPowerCommandMessageId::StateSet),
    { flags: StateSetFlags, system_state: SystemState },
    { }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct StateSetFlags(u32);
bitflags! {
    impl StateSetFlags: u32 {
        const GRACEFUL = 1 << 0;
    }
}

define_command!(
    "SYSTEM_POWER_STATE_GET",
    StateGet,
    MessageId::SystemPowerManagement(SystemPowerCommandMessageId::StateGet),
    { },
    { system_state: SystemState }
);

define_command!(
    "SYSTEM_POWER_STATE_NOTIFY",
    StateNotify,
    MessageId::SystemPowerManagement(SystemPowerCommandMessageId::StateNotify),
    { notify_enable: NotifyEnable },
    { }
);
