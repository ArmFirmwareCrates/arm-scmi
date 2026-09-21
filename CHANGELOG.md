# Changelog

## 0.2.0

### Breaking changes

- Require `Response` for command responses, fixed-size types can implement `FixedSizedResponse`.
- Replace `SharedMemory::payload<T>()` with `status()` and `read_payload()`.
  `SharedMemory::new()` now requires memory accessible from all cores and threads.
- Make `BaseDiscoverListProtocolResponse` fields private, use `iter()` or `contains_protocol()`.

### Improvements

- Implement `Send` for `SharedMemory`.
- Implement `PartialOrd` and `Ord` for `Version`.
- Replace `paste` with `pastey` and update dependencies.

### Bugfixes

- Fix shared-memory response length validation and handling of status-only error responses.
- Fix variable-length `BASE_DISCOVER_LIST_PROTOCOLS` serialization and validation.
- Fix shared-memory aliasing in transport tests.

## 0.1.0

### Improvements

Initial version, targeting SCMI 4.0 beta0. It supports Base, Power Domain Management, and System
Power Management protocols. Provides `ScmiAgent` with synchronous protocol interfaces and the
`Transport` trait and shared-memory transport with a configurable doorbell. Adds common types and
the `define_protocol!` and `define_command!` macros.
