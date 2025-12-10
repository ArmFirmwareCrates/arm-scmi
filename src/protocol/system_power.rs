// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::protocol::define_protocol;

define_protocol!(
    SystemPowerManagement,
    SystemPowerCommandMessageId {
        SystemPowerStateSet = 0x3,
        SystemPowerStateGet = 0x4,
        SystemPowerStateNotify = 0x5,
    }
);
