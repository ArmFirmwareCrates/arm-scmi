// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::protocol::{Command, MessageId, define_protocol};
use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout};

define_protocol!(
    PowerDomainManagement,
    PowerDomainCommandMessageId {
        PowerDomainAttributes = 0x3,
        PowerStateSet = 0x4,
        PowerStateGet = 0x5,
        PowerStateNotify = 0x6,
        PowerStateChangeRequestedNofity = 0x7,
        PowerDomainNameGet = 0x8,
    }
);

#[derive(Eq, FromBytes, Immutable, IntoBytes, KnownLayout, PartialEq)]
#[repr(C, align(4))]
pub struct PowerStateSetReq {
    pub flags: u32,
    pub domain_id: u32,
    pub power_state: u32,
}

impl Command for PowerStateSetReq {
    const ID: MessageId =
        MessageId::PowerDomainManagement(PowerDomainCommandMessageId::PowerStateSet);
    type Response = PowerStateSetResp;
}

#[derive(Eq, FromBytes, Immutable, IntoBytes, KnownLayout, PartialEq)]
#[repr(C, align(4))]
pub struct PowerStateSetResp {
    status: i32,
}
