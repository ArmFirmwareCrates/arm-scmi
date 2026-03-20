// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Error,
    protocol::{MessageId, NotifyEnable, define_command, define_protocol},
};
use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Message attributes of `SYSTEM_POWER_STATE_SET`.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, Immutable, IntoBytes, KnownLayout,
)]
#[repr(transparent)]
pub struct SystemPowerStateSetAttributes(u32);

bitflags! {
    impl SystemPowerStateSetAttributes: u32 {
        /// System warm reset is supported.
        const SYSTEM_WARM_RESET_SUPPORT = 1 << 31;
        /// System suspend is supported.
        const SYSTEM_SUSPEND_SUPPORT = 1 << 30;
    }
}

/// `SYSTEM_POWER_STATE_SET` flags.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct SystemPowerStateSetFlags(u32);
bitflags! {
    impl SystemPowerStateSetFlags: u32 {
        /// The request is a graceful request.
        const GRACEFUL = 1 << 0;
    }
}

/// Standard System State.
#[derive(Clone, Copy, Debug, Eq, IntoPrimitive, PartialEq, TryFromPrimitive)]
#[num_enum(error_type(name = Error, constructor = Error::InvalidSystemState))]
#[repr(u32)]
pub enum StandardSystemState {
    SystemShutdown = 0x0000_0000,
    SystemColdReset = 0x0000_0001,
    SystemWarmReset = 0x0000_0002,
    SystemPowerUp = 0x0000_0003,
    SystemSuspend = 0x0000_0004,
}

/// Vendor specific system state.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct VendorSpecificSystemState(u32);

impl VendorSpecificSystemState {
    const VENDOR_SPEC_BASE: u32 = 0x80000000;

    /// Creates new instance. The valid state range is 0x8000_0000-0xffff_ffff.
    pub const fn new(state: u32) -> Self {
        assert!(Self::is_vendor_specific(state));

        Self(state)
    }

    /// Checks if the value is a vendor specific state number.
    const fn is_vendor_specific(state: u32) -> bool {
        state >= Self::VENDOR_SPEC_BASE
    }
}

impl TryFrom<u32> for VendorSpecificSystemState {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if Self::is_vendor_specific(value) {
            Ok(Self(value))
        } else {
            Err(Error::InvalidSystemState(value))
        }
    }
}

impl From<VendorSpecificSystemState> for u32 {
    fn from(value: VendorSpecificSystemState) -> Self {
        value.0
    }
}

/// Standard or vendor specific system state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SystemState {
    Standard(StandardSystemState),
    VendorSpecific(VendorSpecificSystemState),
}

impl TryFrom<RawSystemState> for SystemState {
    type Error = Error;

    fn try_from(value: RawSystemState) -> Result<Self, Self::Error> {
        Ok(
            if let Ok(value) = VendorSpecificSystemState::try_from(value.0) {
                Self::VendorSpecific(value)
            } else {
                Self::Standard(StandardSystemState::try_from(value.0)?)
            },
        )
    }
}

impl From<SystemState> for RawSystemState {
    fn from(value: SystemState) -> Self {
        Self(match value {
            SystemState::Standard(standard_system_state) => standard_system_state.into(),
            SystemState::VendorSpecific(vendor_specific_system_state) => {
                vendor_specific_system_state.into()
            }
        })
    }
}

/// Raw System State that follows the wire representation of the `system_state` field definition.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct RawSystemState(pub(crate) u32);

define_protocol!(
    SystemPowerManagement,
    u32,
    SystemPowerCommandMessageId {
        StateSet = 0x3,
        StateGet = 0x4,
        StateNotify = 0x5,
    }
);

define_command!(
    "SYSTEM_POWER_STATE_SET",
    SystemPowerStateSet,
    MessageId::SystemPowerManagement(SystemPowerCommandMessageId::StateSet),
    { flags: SystemPowerStateSetFlags, system_state: RawSystemState },
    { }
);

define_command!(
    "SYSTEM_POWER_STATE_GET",
    SystemPowerStateGet,
    MessageId::SystemPowerManagement(SystemPowerCommandMessageId::StateGet),
    { },
    { system_state: RawSystemState }
);

define_command!(
    "SYSTEM_POWER_STATE_NOTIFY",
    SystemPowerStateNotify,
    MessageId::SystemPowerManagement(SystemPowerCommandMessageId::StateNotify),
    { notify_enable: NotifyEnable },
    { }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_state() {
        let state = SystemState::Standard(StandardSystemState::SystemSuspend);
        let raw_state = RawSystemState::from(state);
        assert_eq!(raw_state.0, 0x4);

        let state = SystemState::VendorSpecific(VendorSpecificSystemState::new(0x8000_abcd));
        let raw_state = RawSystemState::from(state);
        assert_eq!(raw_state.0, 0x8000_abcd);
    }
}
