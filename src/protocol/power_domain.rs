// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::protocol::{MessageId, NotifyEnable, define_command, define_protocol};
use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

define_protocol!(
    PowerDomainManagement,
    PowerDomainCommandMessageId {
        DomainAttributes = 0x3,
        StateSet = 0x4,
        StateGet = 0x5,
        StateNotify = 0x6,
        StateChangeRequestedNofity = 0x7,
        DomainNameGet = 0x8,
    }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct DomainId(u32);

impl DomainId {
    pub fn new(id: u16) -> Self {
        Self(id as u32)
    }

    pub fn id(&self) -> u16 {
        self.0 as u16
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct PowerState(u32);

impl PowerState {
    const STATE_TYPE_MASK: u32 = 0x1;
    const STATE_TYPE_SHIFT: u32 = 30;

    const STATE_ID_MASK: u32 = 0x07ff_ffff;
    const STATE_ID_SHIFT: u32 = 0;

    pub const STATE_TYPE_PRESERVED: u32 = 0;
    pub const STATE_TYPE_LOST: u32 = 1;

    pub fn new(state_type: u32, state_id: u32) -> Self {
        assert_eq!(state_type, state_type & Self::STATE_TYPE_MASK);
        assert_eq!(state_id, state_id & Self::STATE_ID_MASK);

        Self(state_type << Self::STATE_TYPE_SHIFT | state_id << Self::STATE_ID_SHIFT)
    }

    pub fn state_type(&self) -> u32 {
        (self.0 >> Self::STATE_TYPE_SHIFT) & Self::STATE_TYPE_MASK
    }
    pub fn state_id(&self) -> u32 {
        (self.0 >> Self::STATE_ID_SHIFT) & Self::STATE_ID_MASK
    }
}

define_command!(
    "POWER_DOMAIN_ATTRIBUTES",
    PowerDomainAttributes,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::DomainAttributes),
    { domain_id: DomainId },
    {
        attributes: DomainAttributes,
        name: [u8; 16]
    }
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct DomainAttributes(u32);
bitflags! {
    impl DomainAttributes: u32 {
        const CHANGE_NOTIFICATIONS_SUPPORT = 1 << 31;
        const ASYNC_SUPPORT = 1 << 30;
        const SYNC_SUPPORT = 1 << 29;
        const CHANGE_REQUESTED_NOTIFICATIONS_SUPPORT = 1 << 28;
        const EXTENDED_POWER_DOMAIN = 1 << 27;
    }
}

define_command!(
    "POWER_STATE_SET",
    StateSet,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateSet),
    {
        flags: StateSetFlags,
        domain_id: DomainId,
        power_state: PowerState
    },
    {}
);

#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(transparent)]
pub struct StateSetFlags(u32);
bitflags! {
    impl StateSetFlags: u32 {
        const ASYNC_TRANSITION = 1 << 0;
    }
}

define_command!(
    "POWER_STATE_GET",
    StateGet,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateGet),
    { domain_id: DomainId },
    { state: PowerState }
);

define_command!(
    "POWER_STATE_NOTIFY",
    StateNotify,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateNotify),
    { domain_id: DomainId, notify_enable: NotifyEnable },
    { }
);

define_command!(
    "POWER_STATE_CHANGE_REQUESTED_NOTIFY",
    StateChangeRequestedNofity,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateChangeRequestedNofity),
    { domain_id: DomainId, notify_enable: NotifyEnable },
    { }
);

define_command!(
    "POWER_DOMAIN_NAME_GET",
    DomainNameGet,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::DomainNameGet),
    { domain_id: DomainId },
    { flags: u32, ext_name: [u8; 64] }
);
