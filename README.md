# Arm System Control and Management Interface library

This library provides support for the
[Arm System Control and Management Interface](https://developer.arm.com/documentation/den0056/fb/)
(SCMI). The implementation is based on version 4.0 beta 0, and all section references correspond to
this version.

The crate includes primitives for defining protocols and implements several standard protocols. It
defines all required command, response, and miscellaneous types. In addition, it offers a high-level
interface for SCMI calls, built on top of an abstract Transport layer. The library also includes
implementations of transport layers.


## Implemented features

* Protocols
  * Base
  * Power Domain Management
  * System Power Management
* Transport layers
  * Shared memory based transport

## Future plans

* Implement further protocols
* Add further transport layers
* Add support for asynchronous calls
* Add support for notifications

## License

The project is MIT and Apache-2.0 dual licensed, see `LICENSE-Apache-2.0` and `LICENSE-MIT`.

## Maintainers

arm-scmi is a trustedfirmware.org maintained project. All contributions are ultimately merged by the maintainers
listed below.

* Bálint Dobszay <balint.dobszay@arm.com>
  [balint-dobszay-arm](https://github.com/balint-dobszay-arm)
* Imre Kis <imre.kis@arm.com>
  [imre-kis-arm](https://github.com/imre-kis-arm)
* Sandrine Afsa <sandrine.afsa@arm.com>
  [sandrine-bailleux-arm](https://github.com/sandrine-bailleux-arm)

## Contributing

Please follow the directions of the [Trusted Firmware Processes](https://trusted-firmware-docs.readthedocs.io/en/latest/generic_processes/index.html)

Contributions are handled through [review.trustedfirmware.org](https://review.trustedfirmware.org/q/project:arm-firmware-crates/arm-scmi).

## Arm trademark notice

Arm is a registered trademark of Arm Limited (or its subsidiaries or affiliates).

This project uses some of the Arm product, service or technology trademarks, as listed in the
[Trademark List][1], in accordance with the [Arm Trademark Use Guidelines][2].

Subsequent uses of these trademarks throughout this repository do not need to be prefixed with the
Arm word trademark.

[1]: https://www.arm.com/company/policies/trademarks/arm-trademark-list
[2]: https://www.arm.com/company/policies/trademarks/guidelines-trademarks

--------------

*Copyright The arm-scmi Contributors.*
