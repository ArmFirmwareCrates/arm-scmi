// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::{
    Error,
    protocol::{MessageId, NotifyEnable, define_command, define_protocol, get_ascii_string},
};
use bitflags::bitflags;
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

/// Power domain protocol attributes flags.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, Immutable, IntoBytes, KnownLayout,
)]
#[repr(transparent)]
pub struct PowerDomainProtocolAttributesFlags(u32);

impl PowerDomainProtocolAttributesFlags {
    const DOMAINS_MASK: u32 = 0xffff;
    const DOMAINS_SHIFT: u32 = 0;

    /// Sets the number of power domains.
    pub const fn set_domain_count(&mut self, count: usize) {
        assert!(count as u32 & !Self::DOMAINS_MASK == 0);

        self.0 &= !(Self::DOMAINS_MASK << Self::DOMAINS_SHIFT);
        self.0 |= ((count as u32) & Self::DOMAINS_MASK) << Self::DOMAINS_SHIFT;
    }

    /// Returns the number of power domains.
    pub const fn domain_count(&self) -> usize {
        ((self.0 >> Self::DOMAINS_SHIFT) & Self::DOMAINS_MASK) as usize
    }
}

/// Power domain protocol attributes.
#[derive(Clone, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout)]
#[repr(C, align(4))]
pub struct PowerDomainProtocolAttributes {
    pub attributes: PowerDomainProtocolAttributesFlags,
    pub statistics_address_low: u32,
    pub statistics_address_high: u32,
    pub statistics_len: u32,
}

/// Power domain ID.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct PowerDomainId(u32);

impl PowerDomainId {
    /// Creates new instance.
    pub const fn new(id: u16) -> Self {
        Self(id as u32)
    }

    /// Return the raw domain ID value.
    pub const fn id(&self) -> u16 {
        self.0 as u16
    }
}

impl From<u16> for PowerDomainId {
    fn from(value: u16) -> Self {
        Self::new(value)
    }
}

/// Power domain attributes.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct DomainAttributes(u32);
bitflags! {
    impl DomainAttributes: u32 {
        /// Power state change notifications are supported on this domain.
        const CHANGE_NOTIFICATIONS_SUPPORT = 1 << 31;
        /// Power state can be set asynchronously.
        const ASYNC_SUPPORT = 1 << 30;
        /// Power state can be set synchronously.
        const SYNC_SUPPORT = 1 << 29;
        /// Power state change requested notifications are supported on this domain.
        const CHANGE_REQUESTED_NOTIFICATIONS_SUPPORT = 1 << 28;
        /// Power domain name is greater than 16 bytes.
        const EXTENDED_POWER_DOMAIN = 1 << 27;
    }
}

/// `POWER_STATE_SET` flags.
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct PowerStateSetFlags(u32);

bitflags! {
    impl PowerStateSetFlags: u32 {
        /// Power transition must be done asynchronously.
        const ASYNC = 1 << 0;
    }
}

/// Power state type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerStateType {
    ContextPreserved,
    ContextLost,
}

impl PowerStateType {
    const STATE_TYPE_BIT: u32 = 1 << 30;
}

impl From<u32> for PowerStateType {
    fn from(value: u32) -> Self {
        if value & Self::STATE_TYPE_BIT == 0 {
            Self::ContextPreserved
        } else {
            Self::ContextLost
        }
    }
}

impl From<PowerStateType> for u32 {
    fn from(value: PowerStateType) -> Self {
        match value {
            PowerStateType::ContextPreserved => 0,
            PowerStateType::ContextLost => PowerStateType::STATE_TYPE_BIT,
        }
    }
}

/// Power State value.
///
/// See Table 6: Power State Parameter Layout for Device Power Domains
#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, FromBytes, IntoBytes, Immutable, KnownLayout,
)]
#[repr(transparent)]
pub struct PowerState(u32);

impl PowerState {
    const MBZ_BITS: u32 = 0xb000_0000;

    const STATE_ID_MASK: u32 = 0x0fff_ffff;
    const STATE_ID_SHIFT: u32 = 0;

    /// Creates new instance.
    pub fn new(state_type: PowerStateType, state_id: u32) -> Self {
        assert_eq!(state_id, state_id & Self::STATE_ID_MASK);

        Self(u32::from(state_type) | state_id << Self::STATE_ID_SHIFT)
    }

    /// Returns the type of the state.
    pub fn state_type(&self) -> PowerStateType {
        self.0.into()
    }

    /// Returns the state ID.
    pub fn state_id(&self) -> u32 {
        (self.0 >> Self::STATE_ID_SHIFT) & Self::STATE_ID_MASK
    }
}

impl TryFrom<u32> for PowerState {
    type Error = Error;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if (value & Self::MBZ_BITS) != 0 {
            return Err(Error::InvalidPowerState(value));
        }

        Ok(Self(value))
    }
}

define_protocol!(
    PowerDomainManagement,
    PowerDomainProtocolAttributes,
    PowerDomainCommandMessageId {
        DomainAttributes = 0x3,
        StateSet = 0x4,
        StateGet = 0x5,
        StateNotify = 0x6,
        StateChangeRequestedNotify = 0x7,
        DomainNameGet = 0x8,
    }
);

define_command!(
    "POWER_DOMAIN_ATTRIBUTES",
    PowerDomainAttributes,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::DomainAttributes),
    { domain_id: PowerDomainId },
    {
        attributes: DomainAttributes,
        name: [u8; 16]
    }
);

impl PowerDomainAttributesResponse {
    /// Returns the power domain name.
    pub fn name(&self) -> Option<&str> {
        get_ascii_string(&self.name)
    }
}

define_command!(
    "POWER_STATE_SET",
    PowerStateSet,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateSet),
    {
        flags: PowerStateSetFlags,
        domain_id: PowerDomainId,
        power_state: PowerState
    },
    {}
);

define_command!(
    "POWER_STATE_GET",
    PowerStateGet,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateGet),
    { domain_id: PowerDomainId },
    { power_state: PowerState }
);

define_command!(
    "POWER_STATE_NOTIFY",
    PowerStateNotify,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateNotify),
    {
        domain_id: PowerDomainId,
        notify_enable: NotifyEnable
    },
    { }
);

define_command!(
    "POWER_STATE_CHANGE_REQUESTED_NOTIFY",
    PowerStateChangeRequestedNotify,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::StateChangeRequestedNotify),
    {
        domain_id: PowerDomainId,
        notify_enable: NotifyEnable
    },
    { }
);

define_command!(
    "POWER_DOMAIN_NAME_GET",
    PowerDomainNameGet,
    MessageId::PowerDomainManagement(PowerDomainCommandMessageId::DomainNameGet),
    { domain_id: PowerDomainId },
    {
        flags: u32,
        ext_name: [u8; 64]
    }
);

impl PowerDomainNameGetResponse {
    /// Returns the power domain name.
    pub fn ext_name(&self) -> Option<&str> {
        get_ascii_string(&self.ext_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_domain_protocol_attributes() {
        let mut attributes = PowerDomainProtocolAttributesFlags(0x8fff);

        assert_eq!(attributes.domain_count(), 0x8fff);

        attributes.set_domain_count(0xcdef);
        assert_eq!(attributes.0, 0xcdef);
    }

    #[test]
    fn domain_id() {
        let domain = PowerDomainId::new(0xabcd);

        assert_eq!(domain.0, 0xabcd);
        assert_eq!(domain.id(), 0xabcd);

        let domain = PowerDomainId::from(0x1234);
        assert_eq!(domain.id(), 0x1234);
    }

    #[test]
    fn power_state() {
        let state = PowerState::new(PowerStateType::ContextPreserved, 0x09ab_cdef);

        assert_eq!(state.state_type(), PowerStateType::ContextPreserved);
        assert_eq!(state.state_id(), 0x09ab_cdef);

        let state = PowerState::new(PowerStateType::ContextLost, 0x09ab_cdef);
        assert_eq!(state.0, 0x49ab_cdef);

        let state = PowerState::try_from(0x4789_7654).unwrap();
        assert_eq!(state.state_type(), PowerStateType::ContextLost);
        assert_eq!(state.state_id(), 0x0789_7654);

        assert!(PowerState::try_from(0x8000_0000).is_err());
    }

    #[test]
    fn name() {
        let mut response = PowerDomainAttributesResponse {
            attributes: DomainAttributes::default(),
            name: [b'A'; 16],
        };

        response.name[..6].copy_from_slice(b"Hello\0");
        assert_eq!(response.name(), Some("Hello"));
    }

    #[test]
    fn ext_name() {
        let mut response = PowerDomainNameGetResponse {
            flags: 0,
            ext_name: [b'A'; 64],
        };

        response.ext_name[..6].copy_from_slice(b"Hello\0");
        assert_eq!(response.ext_name(), Some("Hello"));
    }
}
