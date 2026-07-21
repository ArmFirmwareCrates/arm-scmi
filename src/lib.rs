// SPDX-FileCopyrightText: Copyright The arm-scmi Contributors.
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg_attr(not(test), no_std)]
#![doc = include_str!("../README.md")]
#![deny(clippy::undocumented_unsafe_blocks)]
#![deny(unsafe_op_in_unsafe_fn)]

/// Common protocol elements.
pub mod protocol;
/// Transport trait and implementations.
pub mod transport;

use crate::{
    protocol::{
        MessageId, NotifyEnable, ProtocolId, StandardProtocolId, StatusCode, Version,
        base::{
            self, BaseCommandMessageId, BaseDiscoverAgent, BaseDiscoverAgentResponse,
            BaseDiscoverImplementationVersion, BaseDiscoverListProtocol,
            BaseDiscoverListProtocolResponse, BaseDiscoverSubVendor, BaseDiscoverSubVendorResponse,
            BaseDiscoverVendor, BaseDiscoverVendorResponse, BaseNotifyErrors, BasePermissionFlags,
            BaseProtocolAttributes, BaseResetAgentConfiguration, BaseResetAgentConfigurationFlags,
            BaseSetDevicePermission, BaseSetProtocolPermissions,
        },
        power_domain::{
            self, PowerDomainAttributes, PowerDomainAttributesResponse,
            PowerDomainCommandMessageId, PowerDomainNameGet, PowerDomainNameGetResponse,
            PowerDomainProtocolAttributes, PowerState, PowerStateChangeRequestedNotify,
            PowerStateGet, PowerStateNotify, PowerStateSet, PowerStateSetFlags,
        },
        system_power::{
            self, SystemPowerCommandMessageId, SystemPowerStateGet, SystemPowerStateNotify,
            SystemPowerStateSet, SystemPowerStateSetFlags, SystemState,
        },
    },
    transport::Transport,
};
use thiserror::Error;

/// Rich error types returned by this module.
#[derive(Clone, Copy, Debug, Error, PartialEq, Eq)]
pub enum Error {
    #[error("Status code {0:?}")]
    Status(StatusCode),
    #[error("Invalid status code {0}")]
    InvalidStatusCode(i32),
    #[error("Invalid protocol ID {0}")]
    InvalidProtocolId(u8),
    #[error("Invalid message type {0}")]
    InvalidMessageType(u8),
    #[error("Invalid message ID {0}")]
    InvalidMessageId(u8),
    #[error("invalid power state {0}")]
    InvalidPowerState(u32),
    #[error("Invalid system state {0}")]
    InvalidSystemState(u32),
    #[error("Protocol not supported")]
    ProtocolNotSupported,
    #[error("Channel communication error")]
    ChannelError,
    #[error("Length overflow")]
    LengthOverflow,
    #[error("Payload exceeds maximum size")]
    PayloadExceedsMaxSize,
    #[error("Response too short")]
    ResponseTooShort,
    #[error("Unexpected response {0:?}")]
    UnexpectedResponse(MessageId),
    #[error("Unexpected token {0:?}")]
    UnexpectedToken(u16),
}

/// SCMI agent.
pub struct ScmiAgent<T: Transport> {
    transport: T,
    power_domain_supported: bool,
    system_power_supported: bool,
}

impl<T: Transport> ScmiAgent<T> {
    /// Creates new instance.
    pub fn new(mut transport: T) -> Result<Self, Error> {
        // The Base protocol is mandatory. Other protocols are optional and must be discovered.
        let mut base = ScmiBase {
            transport: &mut transport,
        };

        let protocols = base.discover_list_protocols(0)?;

        Ok(Self {
            transport,
            power_domain_supported: protocols.contains_protocol(ProtocolId::Standard(
                StandardProtocolId::PowerDomainManagement,
            )),
            system_power_supported: protocols.contains_protocol(ProtocolId::Standard(
                StandardProtocolId::SystemPowerManagement,
            )),
        })
    }

    /// Returns Base protocol agent.
    pub fn base(&mut self) -> ScmiBase<'_, T> {
        ScmiBase {
            transport: &mut self.transport,
        }
    }

    /// Returns Power Domain Management protocol agent.
    pub fn power_domain_management(&mut self) -> Result<ScmiPowerDomainManagement<'_, T>, Error> {
        if !self.power_domain_supported {
            return Err(Error::ProtocolNotSupported);
        }

        Ok(ScmiPowerDomainManagement {
            transport: &mut self.transport,
        })
    }

    /// Returns System Power Management protocol agent.
    pub fn system_power_management(&mut self) -> Result<ScmiSystemPowerManagement<'_, T>, Error> {
        if !self.system_power_supported {
            return Err(Error::ProtocolNotSupported);
        }

        Ok(ScmiSystemPowerManagement {
            transport: &mut self.transport,
        })
    }
}

/// SCMI Base protocol agent.
pub struct ScmiBase<'a, T: Transport> {
    transport: &'a mut T,
}

impl<'a, T: Transport> ScmiBase<'a, T> {
    /// An agent can discover its own agent ID, by calling [Self::discover_agent()] with this ID.
    /// The `agent_id` field of the response will contain the agent ID of the caller.
    pub const AGENT_ID_SELF_DISCOVERY: u32 = 0xffff_ffff;

    /// Returns the protocol version.
    pub fn protocol_version(&mut self) -> Result<Version, Error> {
        let response = self.transport.invoke_command(base::ProtocolVersion {})?;

        Ok(response.version)
    }

    /// Negotiates the protocol version that the agent intends to use.
    pub fn negotiate_protocol_version(&mut self, version: Version) -> Result<(), Error> {
        self.transport
            .invoke_command(base::NegotiateProtocolVersion { version })?;

        Ok(())
    }

    /// Returns the implementation details that are associated with the protocol.
    pub fn protocol_attributes(&mut self) -> Result<BaseProtocolAttributes, Error> {
        let response = self.transport.invoke_command(base::ProtocolAttributes {})?;

        Ok(response.attributes)
    }

    /// Returns the implementation details associated with a specific message of the protocol.
    pub fn protocol_message_attributes(
        &mut self,
        message_id: BaseCommandMessageId,
    ) -> Result<u32, Error> {
        let response = self
            .transport
            .invoke_command(base::ProtocolMessageAttributes {
                message_id: u8::from(message_id).into(),
            })?;

        Ok(response.attributes)
    }

    /// Returns the vendor identifier.
    pub fn discover_vendor(&mut self) -> Result<BaseDiscoverVendorResponse, Error> {
        self.transport.invoke_command(BaseDiscoverVendor {})
    }

    /// Returns the sub vendor identifier.
    pub fn discover_sub_vendor(&mut self) -> Result<BaseDiscoverSubVendorResponse, Error> {
        self.transport.invoke_command(BaseDiscoverSubVendor {})
    }

    /// Returns the vendor specific implementation version.
    pub fn implementation_version(&mut self) -> Result<u32, Error> {
        let response = self
            .transport
            .invoke_command(BaseDiscoverImplementationVersion {})?;

        Ok(response.implementation_version)
    }

    /// Returns the protocol list that is accessible by the agent.
    ///
    /// `skip` indicates the number of protocols to skipped, i.e. the index of the first
    /// protocol to be included in the response.
    pub fn discover_list_protocols(
        &mut self,
        skip: u32,
    ) -> Result<BaseDiscoverListProtocolResponse, Error> {
        self.transport
            .invoke_command(BaseDiscoverListProtocol { skip })
    }

    /// Returns the name and ID of the agent.
    pub fn discover_agent(&mut self, agent_id: u32) -> Result<BaseDiscoverAgentResponse, Error> {
        self.transport
            .invoke_command(BaseDiscoverAgent { agent_id })
    }

    /// Enables/disables error notification.
    pub fn notify_errors(&mut self, enable: bool) -> Result<(), Error> {
        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, enable);

        self.transport
            .invoke_command(BaseNotifyErrors { notify_enable })?;
        Ok(())
    }

    /// Indicates to the platform whether an agent has permissions to access devices.
    pub fn set_device_permissions(
        &mut self,
        agent_id: u32,
        device_id: u32,
        allow: bool,
    ) -> Result<(), Error> {
        let mut flags = BasePermissionFlags::empty();
        flags.set(BasePermissionFlags::ACCESS_TYPE, allow);

        self.transport.invoke_command(BaseSetDevicePermission {
            agent_id,
            device_id,
            flags,
        })?;
        Ok(())
    }

    /// Indicate to the platform whether an agent has permissions to use a protocol to access the
    /// platform resources that are associated with a specific device.
    pub fn set_protocol_permissions(
        &mut self,
        agent_id: u32,
        device_id: u32,
        protocol_id: ProtocolId,
        allow: bool,
    ) -> Result<(), Error> {
        let mut flags = BasePermissionFlags::empty();
        flags.set(BasePermissionFlags::ACCESS_TYPE, allow);

        self.transport.invoke_command(BaseSetProtocolPermissions {
            agent_id,
            device_id,
            command_id: protocol_id.into(),
            flags,
        })?;
        Ok(())
    }

    /// Resets platform resource settings that were previously configured by an agent.
    pub fn reset_agent_configuration(
        &mut self,
        agent_id: u32,
        flags: BaseResetAgentConfigurationFlags,
    ) -> Result<(), Error> {
        self.transport
            .invoke_command(BaseResetAgentConfiguration { agent_id, flags })?;
        Ok(())
    }
}

/// SCMI Power Domain Management protocol agent.
pub struct ScmiPowerDomainManagement<'a, T: Transport> {
    transport: &'a mut T,
}

impl<'a, T: Transport> ScmiPowerDomainManagement<'a, T> {
    /// Returns the protocol version.
    pub fn protocol_version(&mut self) -> Result<Version, Error> {
        let response = self
            .transport
            .invoke_command(power_domain::ProtocolVersion {})?;

        Ok(response.version)
    }

    /// Negotiates the protocol version that the agent intends to use.
    pub fn negotiate_protocol_version(&mut self, version: Version) -> Result<(), Error> {
        self.transport
            .invoke_command(power_domain::NegotiateProtocolVersion { version })?;

        Ok(())
    }

    /// Returns the implementation details that are associated with the protocol.
    pub fn protocol_attributes(&mut self) -> Result<PowerDomainProtocolAttributes, Error> {
        let response = self
            .transport
            .invoke_command(power_domain::ProtocolAttributes {})?;

        Ok(response.attributes)
    }

    /// Returns the implementation details associated with a specific message of the protocol.
    pub fn protocol_message_attributes(
        &mut self,
        message_id: PowerDomainCommandMessageId,
    ) -> Result<u32, Error> {
        let response = self
            .transport
            .invoke_command(power_domain::ProtocolMessageAttributes {
                message_id: u8::from(message_id) as u32,
            })?;

        Ok(response.attributes)
    }

    /// Returns the attribute flags associated with a specific power domain.
    pub fn power_domain_attributes(
        &mut self,
        domain_id: u16,
    ) -> Result<PowerDomainAttributesResponse, Error> {
        let response = self.transport.invoke_command(PowerDomainAttributes {
            domain_id: domain_id.into(),
        })?;

        Ok(response)
    }

    /// Sets the power state of a power domain.
    pub fn power_state_set(
        &mut self,
        domain_id: u16,
        power_state: PowerState,
        asynchronous: bool,
    ) -> Result<(), Error> {
        let mut flags = PowerStateSetFlags::empty();
        flags.set(PowerStateSetFlags::ASYNC, asynchronous);

        self.transport.invoke_command(PowerStateSet {
            flags,
            domain_id: domain_id.into(),
            power_state,
        })?;

        Ok(())
    }

    /// Returns the power state of a power domain.
    pub fn power_state_get(&mut self, domain_id: u16) -> Result<PowerState, Error> {
        let response = self.transport.invoke_command(PowerStateGet {
            domain_id: domain_id.into(),
        })?;

        Ok(response.power_state)
    }

    /// Enables/disables `POWER_STATE_CHANGED` notifications.
    pub fn power_state_notify(&mut self, domain_id: u16, enable: bool) -> Result<(), Error> {
        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, enable);

        self.transport.invoke_command(PowerStateNotify {
            domain_id: domain_id.into(),
            notify_enable,
        })?;

        Ok(())
    }

    /// Enables/disables `POWER_STATE_CHANGE_REQUESTED` notifications.
    pub fn power_state_change_requested_notify(
        &mut self,
        domain_id: u16,
        enable: bool,
    ) -> Result<(), Error> {
        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, enable);

        self.transport
            .invoke_command(PowerStateChangeRequestedNotify {
                domain_id: domain_id.into(),
                notify_enable,
            })?;

        Ok(())
    }

    /// Returns the extended name of the power domain.
    pub fn power_domain_name_get(
        &mut self,
        domain_id: u16,
    ) -> Result<PowerDomainNameGetResponse, Error> {
        self.transport.invoke_command(PowerDomainNameGet {
            domain_id: domain_id.into(),
        })
    }
}

/// SCMI System Power Management protocol agent.
pub struct ScmiSystemPowerManagement<'a, T: Transport> {
    transport: &'a mut T,
}

impl<'a, T: Transport> ScmiSystemPowerManagement<'a, T> {
    /// Returns the protocol version.
    pub fn protocol_version(&mut self) -> Result<Version, Error> {
        let response = self
            .transport
            .invoke_command(system_power::ProtocolVersion {})?;

        Ok(response.version)
    }

    /// Negotiates the protocol version that the agent intends to use.
    pub fn negotiate_protocol_version(&mut self, version: Version) -> Result<(), Error> {
        self.transport
            .invoke_command(system_power::NegotiateProtocolVersion { version })?;

        Ok(())
    }

    /// Returns the implementation details that are associated with the protocol.
    pub fn protocol_attributes(&mut self) -> Result<u32, Error> {
        let response = self
            .transport
            .invoke_command(system_power::ProtocolAttributes {})?;

        Ok(response.attributes)
    }

    /// Returns the implementation details associated with a specific message of the protocol.
    pub fn protocol_message_attributes(
        &mut self,
        message_id: SystemPowerCommandMessageId,
    ) -> Result<u32, Error> {
        let response = self
            .transport
            .invoke_command(system_power::ProtocolMessageAttributes {
                message_id: u8::from(message_id) as u32,
            })?;

        Ok(response.attributes)
    }

    /// Sets the system power state.
    pub fn system_power_state_set(
        &mut self,
        system_state: SystemState,
        flags: SystemPowerStateSetFlags,
    ) -> Result<(), Error> {
        self.transport.invoke_command(SystemPowerStateSet {
            flags,
            system_state: system_state.into(),
        })?;

        Ok(())
    }

    /// Returns the system power state.
    pub fn system_power_state_get(&mut self) -> Result<SystemState, Error> {
        let response = self.transport.invoke_command(SystemPowerStateGet {})?;

        response.system_state.try_into()
    }

    /// Enables/disables `SYSTEM_POWER_STATE_NOTIFIER` notifications.
    pub fn system_power_state_notify(&mut self, enable: bool) -> Result<(), Error> {
        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, enable);

        self.transport
            .invoke_command(SystemPowerStateNotify { notify_enable })?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        Command, Response, StandardStatusCode,
        base::{
            BaseDiscoverImplementationVersionResponse, BaseNotifyErrorsResponse,
            BaseResetAgentConfigurationResponse, BaseSetDevicePermissionResponse,
            BaseSetProtocolPermissionsResponse,
        },
        power_domain::{
            DomainAttributes, PowerDomainAttributesResponse, PowerDomainNameGetResponse,
            PowerDomainProtocolAttributes, PowerDomainProtocolAttributesFlags,
            PowerStateChangeRequestedNotifyResponse, PowerStateGetResponse,
            PowerStateNotifyResponse, PowerStateSetResponse, PowerStateType,
        },
        system_power::{
            RawSystemState, StandardSystemState, SystemPowerStateGetResponse,
            SystemPowerStateNotifyResponse, SystemPowerStateSetResponse,
        },
    };
    use core::any::type_name;
    use zerocopy::IntoBytes;

    struct MockCall {
        request_type: &'static str,
        request: Vec<u8>,
        response_type: &'static str,
        response: Result<Vec<u8>, Error>,
    }

    impl MockCall {
        pub fn new<C: Command>(command: C, response: Result<C::Response, Error>) -> Self {
            Self {
                request_type: type_name::<C>(),
                request: command.as_bytes().to_vec(),
                response_type: type_name::<C::Response>(),
                response: response.map(|r| r.as_bytes().unwrap().to_vec()),
            }
        }

        pub fn validate_request<C: Command>(&self, command: C) {
            assert_eq!(self.request_type, type_name::<C>());
            assert_eq!(self.request, command.as_bytes().to_vec());
        }

        pub fn get_response<C: Command>(&self) -> Result<C::Response, Error> {
            assert_eq!(self.response_type, type_name::<C::Response>());

            match &self.response {
                Ok(response) => {
                    let bytes = response.as_bytes();
                    C::Response::from_reader(|buffer| {
                        buffer[0..bytes.len()].copy_from_slice(bytes);
                        bytes.len()
                    })
                }
                Err(error) => Err(*error),
            }
        }
    }

    struct MockTransport {
        pub calls: Vec<MockCall>,
    }

    impl MockTransport {
        pub fn new() -> Self {
            Self { calls: Vec::new() }
        }

        pub fn new_with_base_protocol() -> Self {
            let mut instance = Self { calls: Vec::new() };

            instance.add_expected_call(
                BaseDiscoverListProtocol { skip: 0 },
                Ok(BaseDiscoverListProtocolResponse {
                    protocols: [0; 4],
                    num_protocols: 1,
                }),
            );

            instance
        }

        pub fn new_with_standard_protocol(standard_protocol_id: StandardProtocolId) -> Self {
            let mut instance = Self { calls: Vec::new() };

            instance.add_expected_call(
                BaseDiscoverListProtocol { skip: 0 },
                Ok(BaseDiscoverListProtocolResponse {
                    protocols: [
                        u8::from(ProtocolId::Standard(standard_protocol_id)) as u32,
                        0,
                        0,
                        0,
                    ],
                    num_protocols: 1,
                }),
            );

            instance
        }

        pub fn add_expected_call<C: Command>(
            &mut self,
            command: C,
            response: Result<C::Response, Error>,
        ) {
            self.calls.insert(0, MockCall::new(command, response));
        }
    }

    impl Transport for MockTransport {
        fn invoke_command<C: Command>(&mut self, command: C) -> Result<C::Response, Error> {
            let call = self.calls.pop().unwrap();

            call.validate_request(command);
            call.get_response::<C>()
        }
    }

    #[test]
    fn transport() {
        let mut transport = MockTransport::new();

        transport.add_expected_call(
            BaseDiscoverListProtocol { skip: 0 },
            Ok(BaseDiscoverListProtocolResponse {
                protocols: [0x0000_0011, 0, 0, 0],
                num_protocols: 1,
            }),
        );

        let scmi = ScmiAgent::new(transport);
        assert!(scmi.is_ok());

        let mut scmi = scmi.unwrap();
        assert!(scmi.power_domain_management().is_ok());
        assert!(scmi.system_power_management().is_err());

        let mut transport = MockTransport::new();
        transport.add_expected_call(
            BaseDiscoverListProtocol { skip: 0 },
            Ok(BaseDiscoverListProtocolResponse {
                protocols: [0x0000_0012, 0, 0, 0],
                num_protocols: 1,
            }),
        );

        let scmi = ScmiAgent::new(transport);
        assert!(scmi.is_ok());

        let mut scmi = scmi.unwrap();
        assert!(scmi.power_domain_management().is_err());
        assert!(scmi.system_power_management().is_ok());
    }

    #[test]
    fn base_protocol_version() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            base::ProtocolVersion {},
            Ok(base::ProtocolVersionResponse {
                version: Version::new(0x1234, 0x5678),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(Ok(Version::new(0x1234, 0x5678)), base.protocol_version());
    }

    #[test]
    fn base_protocol_version_error() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            base::ProtocolVersion {},
            Err(Error::Status(StatusCode::Standard(
                StandardStatusCode::NotSupported,
            ))),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(
            Err(Error::Status(StatusCode::Standard(
                StandardStatusCode::NotSupported,
            ))),
            base.protocol_version()
        );
    }

    #[test]
    fn base_negotiate_protocol_version() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            base::NegotiateProtocolVersion {
                version: Version::new(0x1234, 0x5678),
            },
            Ok(base::NegotiateProtocolVersionResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(
            Ok(()),
            base.negotiate_protocol_version(Version::new(0x1234, 0x5678))
        );
    }

    #[test]
    fn base_protocol_attributes() {
        let mut transport = MockTransport::new_with_base_protocol();

        let mut protocol_attributes = BaseProtocolAttributes::default();
        protocol_attributes.set_agent_count(10);
        protocol_attributes.set_protocol_count(20);
        transport.add_expected_call(
            base::ProtocolAttributes {},
            Ok(base::ProtocolAttributesResponse {
                attributes: protocol_attributes,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(Ok(protocol_attributes), base.protocol_attributes());
    }

    #[test]
    fn base_protocol_message_attributes() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            base::ProtocolMessageAttributes { message_id: 0x07 },
            Ok(base::ProtocolMessageAttributesResponse {
                attributes: 0xabcd_ef01,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(
            Ok(0xabcd_ef01),
            base.protocol_message_attributes(BaseCommandMessageId::DiscoverAgent)
        );
    }

    #[test]
    fn base_discover_vendor() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            BaseDiscoverVendor {},
            Ok(BaseDiscoverVendorResponse {
                vendor_identifier: "Hello_World1234\0".as_bytes().try_into().unwrap(),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        let vendor = base.discover_vendor().unwrap();
        assert_eq!(Some("Hello_World1234"), vendor.vendor_identifier());
    }

    #[test]
    fn base_discover_sub_vendor() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            BaseDiscoverSubVendor {},
            Ok(BaseDiscoverSubVendorResponse {
                vendor_identifier: "Hello_World5678\0".as_bytes().try_into().unwrap(),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        let sub_vendor = base.discover_sub_vendor().unwrap();
        assert_eq!(Some("Hello_World5678"), sub_vendor.vendor_identifier());
    }

    #[test]
    fn base_implementation_version() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            BaseDiscoverImplementationVersion {},
            Ok(BaseDiscoverImplementationVersionResponse {
                implementation_version: 0x1234_5678,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(Ok(0x1234_5678), base.implementation_version());
    }

    #[test]
    fn base_discover_protocol_list() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            BaseDiscoverListProtocol { skip: 4 },
            Ok(BaseDiscoverListProtocolResponse {
                protocols: [0x0000_0011, 0, 0, 0],
                num_protocols: 1,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        let protocols = base.discover_list_protocols(4).unwrap();
        assert!(protocols.contains_protocol(ProtocolId::Standard(
            StandardProtocolId::PowerDomainManagement
        )));
    }

    #[test]
    fn base_discover_agent() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            BaseDiscoverAgent { agent_id: 7 },
            Ok(BaseDiscoverAgentResponse {
                agent_id: 7,
                name: b"agent_012345678\0".as_bytes().try_into().unwrap(),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        let agent = base.discover_agent(7).unwrap();
        assert_eq!(7, agent.agent_id);
        assert_eq!(Some("agent_012345678"), agent.name());
    }

    #[test]
    fn base_notify_enable() {
        let mut transport = MockTransport::new_with_base_protocol();

        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, true);
        transport.add_expected_call(
            BaseNotifyErrors { notify_enable },
            Ok(BaseNotifyErrorsResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(Ok(()), base.notify_errors(true));
    }

    #[test]
    fn base_set_device_permissions() {
        let mut transport = MockTransport::new_with_base_protocol();

        let mut device_flags = BasePermissionFlags::empty();
        device_flags.set(BasePermissionFlags::ACCESS_TYPE, true);
        transport.add_expected_call(
            BaseSetDevicePermission {
                agent_id: 3,
                device_id: 4,
                flags: device_flags,
            },
            Ok(BaseSetDevicePermissionResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(Ok(()), base.set_device_permissions(3, 4, true));
    }

    #[test]
    fn base_set_protocol_permissions() {
        let mut transport = MockTransport::new_with_base_protocol();

        let mut protocol_flags = BasePermissionFlags::empty();
        protocol_flags.set(BasePermissionFlags::ACCESS_TYPE, false);
        transport.add_expected_call(
            BaseSetProtocolPermissions {
                agent_id: 5,
                device_id: 6,
                command_id: ProtocolId::Standard(StandardProtocolId::PowerDomainManagement).into(),
                flags: protocol_flags,
            },
            Ok(BaseSetProtocolPermissionsResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(
            Ok(()),
            base.set_protocol_permissions(
                5,
                6,
                ProtocolId::Standard(StandardProtocolId::PowerDomainManagement),
                false
            )
        );
    }

    #[test]
    fn base_reset_agent_configuration() {
        let mut transport = MockTransport::new_with_base_protocol();

        transport.add_expected_call(
            BaseResetAgentConfiguration {
                agent_id: 9,
                flags: BaseResetAgentConfigurationFlags::PERMISSIONS_RESET,
            },
            Ok(BaseResetAgentConfigurationResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut base = scmi.base();

        assert_eq!(
            Ok(()),
            base.reset_agent_configuration(9, BaseResetAgentConfigurationFlags::PERMISSIONS_RESET)
        );
    }

    #[test]
    fn power_domain_protocol_version() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        transport.add_expected_call(
            power_domain::ProtocolVersion {},
            Ok(power_domain::ProtocolVersionResponse {
                version: Version::new(0x1111, 0x2222),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(Ok(Version::new(0x1111, 0x2222)), power.protocol_version());
    }

    #[test]
    fn power_domain_negotiate_protocol_version() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        transport.add_expected_call(
            power_domain::NegotiateProtocolVersion {
                version: Version::new(0x1234, 0x5678),
            },
            Ok(power_domain::NegotiateProtocolVersionResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(
            Ok(()),
            power.negotiate_protocol_version(Version::new(0x1234, 0x5678))
        );
    }

    #[test]
    fn power_domain_protocol_attributes() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        let mut flags = PowerDomainProtocolAttributesFlags::default();
        flags.set_domain_count(6);

        let attributes = PowerDomainProtocolAttributes {
            attributes: flags,
            statistics_address_low: 0x1234_5678,
            statistics_address_high: 0xabcd_ef01,
            statistics_len: 0x2345_6789,
        };

        transport.add_expected_call(
            power_domain::ProtocolAttributes {},
            Ok(power_domain::ProtocolAttributesResponse {
                attributes: attributes.clone(),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(Ok(attributes), power.protocol_attributes());
    }

    #[test]
    fn power_domain_protocol_message_attributes() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        transport.add_expected_call(
            power_domain::ProtocolMessageAttributes { message_id: 0x05 },
            Ok(power_domain::ProtocolMessageAttributesResponse {
                attributes: 0x1234_5678,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(
            Ok(0x1234_5678),
            power.protocol_message_attributes(PowerDomainCommandMessageId::StateGet)
        );
    }

    #[test]
    fn power_domain_attributes() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        transport.add_expected_call(
            PowerDomainAttributes {
                domain_id: 2u16.into(),
            },
            Ok(PowerDomainAttributesResponse {
                attributes: DomainAttributes::SYNC_SUPPORT,
                name: b"power_domain123\0".as_bytes().try_into().unwrap(),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        let response = power.power_domain_attributes(2).unwrap();
        assert!(response.attributes.contains(DomainAttributes::SYNC_SUPPORT));
        assert_eq!(Some("power_domain123"), response.name());
    }

    #[test]
    fn power_state_set() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        let mut flags = PowerStateSetFlags::empty();
        flags.set(PowerStateSetFlags::ASYNC, true);
        let state = PowerState::new(PowerStateType::ContextPreserved, 0x55);
        transport.add_expected_call(
            PowerStateSet {
                flags,
                domain_id: 1u16.into(),
                power_state: state,
            },
            Ok(PowerStateSetResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(Ok(()), power.power_state_set(1, state, true));
    }

    #[test]
    fn power_state_get() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        let power_state = PowerState::new(PowerStateType::ContextLost, 0x123);
        transport.add_expected_call(
            PowerStateGet {
                domain_id: 4u16.into(),
            },
            Ok(PowerStateGetResponse { power_state }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(Ok(power_state), power.power_state_get(4));
    }

    #[test]
    fn power_state_notify() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, true);
        transport.add_expected_call(
            PowerStateNotify {
                domain_id: 7u16.into(),
                notify_enable,
            },
            Ok(PowerStateNotifyResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(Ok(()), power.power_state_notify(7, true));
    }

    #[test]
    fn power_state_change_requested_notify() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, false);
        transport.add_expected_call(
            PowerStateChangeRequestedNotify {
                domain_id: 9u16.into(),
                notify_enable,
            },
            Ok(PowerStateChangeRequestedNotifyResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        assert_eq!(Ok(()), power.power_state_change_requested_notify(9, false));
    }

    #[test]
    fn power_domain_name_get() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::PowerDomainManagement);

        let mut ext_name = [0u8; 64];
        ext_name[..15].copy_from_slice(b"power_domain_01");
        transport.add_expected_call(
            PowerDomainNameGet {
                domain_id: 3u16.into(),
            },
            Ok(PowerDomainNameGetResponse {
                flags: 0x1234_5678,
                ext_name,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut power = scmi.power_domain_management().unwrap();

        let response = power.power_domain_name_get(3).unwrap();
        assert_eq!(0x1234_5678, response.flags);
        assert_eq!(Some("power_domain_01"), response.ext_name());
    }

    #[test]
    fn system_power_protocol_version() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::SystemPowerManagement);

        transport.add_expected_call(
            system_power::ProtocolVersion {},
            Ok(system_power::ProtocolVersionResponse {
                version: Version::new(0xaaaa, 0xbbbb),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut system_power = scmi.system_power_management().unwrap();

        assert_eq!(
            Ok(Version::new(0xaaaa, 0xbbbb)),
            system_power.protocol_version()
        );
    }

    #[test]
    fn system_power_negotiate_protocol_version() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::SystemPowerManagement);

        transport.add_expected_call(
            system_power::NegotiateProtocolVersion {
                version: Version::new(0x1234, 0x5678),
            },
            Ok(system_power::NegotiateProtocolVersionResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut system_power = scmi.system_power_management().unwrap();

        assert_eq!(
            Ok(()),
            system_power.negotiate_protocol_version(Version::new(0x1234, 0x5678))
        );
    }

    #[test]
    fn system_power_protocol_attributes() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::SystemPowerManagement);

        transport.add_expected_call(
            system_power::ProtocolAttributes {},
            Ok(system_power::ProtocolAttributesResponse {
                attributes: 0xabcd_ef01,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut system_power = scmi.system_power_management().unwrap();

        assert_eq!(Ok(0xabcd_ef01), system_power.protocol_attributes());
    }

    #[test]
    fn system_power_protocol_message_attributes() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::SystemPowerManagement);

        transport.add_expected_call(
            system_power::ProtocolMessageAttributes { message_id: 0x03 },
            Ok(system_power::ProtocolMessageAttributesResponse {
                attributes: 0x5678_9abc,
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut system_power = scmi.system_power_management().unwrap();

        assert_eq!(
            Ok(0x5678_9abc),
            system_power.protocol_message_attributes(SystemPowerCommandMessageId::StateSet)
        );
    }

    #[test]
    fn system_power_state_set() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::SystemPowerManagement);

        let mut flags = SystemPowerStateSetFlags::empty();
        flags.set(SystemPowerStateSetFlags::GRACEFUL, true);
        transport.add_expected_call(
            SystemPowerStateSet {
                flags,
                system_state: RawSystemState(0x4),
            },
            Ok(SystemPowerStateSetResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut system_power = scmi.system_power_management().unwrap();

        assert_eq!(
            Ok(()),
            system_power.system_power_state_set(
                SystemState::Standard(StandardSystemState::SystemSuspend),
                flags
            )
        );
    }

    #[test]
    fn system_power_state_get() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::SystemPowerManagement);

        transport.add_expected_call(
            SystemPowerStateGet {},
            Ok(SystemPowerStateGetResponse {
                system_state: RawSystemState(0x3),
            }),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut system_power = scmi.system_power_management().unwrap();

        assert_eq!(
            Ok(SystemState::Standard(StandardSystemState::SystemPowerUp)),
            system_power.system_power_state_get()
        );
    }

    #[test]
    fn system_power_state_notify() {
        let mut transport =
            MockTransport::new_with_standard_protocol(StandardProtocolId::SystemPowerManagement);

        let mut notify_enable = NotifyEnable::empty();
        notify_enable.set(NotifyEnable::ENABLE, true);
        transport.add_expected_call(
            SystemPowerStateNotify { notify_enable },
            Ok(SystemPowerStateNotifyResponse {}),
        );

        let mut scmi = ScmiAgent::new(transport).unwrap();
        let mut system_power = scmi.system_power_management().unwrap();

        assert_eq!(Ok(()), system_power.system_power_state_notify(true));
    }
}
