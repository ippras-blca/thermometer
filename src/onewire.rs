//! RMT-based Onewire Implementation
//!
//! The Onewire module driver can be used to communicate with onewire (1-Wire)
//! devices.
//!
//! This module is an abstraction around the esp-idf component [onewire_bus](https://components.espressif.com/components/espressif/onewire_bus)
//! implementation. It is recommended to read the usage of the C API in this [example](https://github.com/espressif/esp-idf/tree/v5.2.2/examples/peripherals/rmt/onewire)
//!
//!
//! This implementation currently supports the one-wire API from the new (v5) esp-idf API.
//!
//! The pin this peripheral is attached to must be
//! externally pulled-up with a 4.7kOhm resistor.
//!
//! todo:
//!  - crc checking on messages
//!  - helper methods on the driver for executing commands
//!
//! See the `examples/` folder of this repository for more.

use core::{marker::PhantomData, ptr};

/// Calculates the crc8 of the input data.
/// Checks to see if data (including the crc byte) passes the crc check.
///
/// `CRC = X^8 + X^5 + X^4 + X^0`
///
/// A nice property of this crc8 algorithm is that if you include the crc value
/// in the data it will always return 0, so it's not needed to separate the data
/// from the crc value
fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0;
    for byte in data {
        crc ^= byte;
        for _ in 0..u8::BITS {
            let bit = crc & 0x01;
            crc >>= 1;
            if bit != 0 {
                // 0b1000_1100
                crc ^= 0x8C;
            }
        }
    }
    crc
}

trait Check {
    fn check(&self) -> Result<(), u8>;
}

// impl Check for OWAddress {
//     fn check(&self) -> Result<(), u8> {}
// }

/// Onewire Address type
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct OWAddress(u64);

impl OWAddress {
    pub fn address(&self) -> u64 {
        self.0
    }

    // pub fn serial_number(&self) -> u64 {
    //     self.0 >> 8 && 0xff
    // }

    pub fn family_code(&self) -> u8 {
        (self.0 & 0xff) as u8
    }

    pub fn crc(&self) -> u8 {
        (self.0 & 0xff << 56) as u8
    }
}

/// Command codes
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum OWCommand {
    Search = 0xF0,      //Obtain IDs of all devices on the bus
    MatchRom = 0x55,    //Address specific device
    SkipRom = 0xCC,     //Skip addressing
    ReadRom = 0x33,     //Identification
    SearchAlarm = 0xEC, // Conditional search for all devices in an alarm state.
    ReadPowerSupply = 0xB4,
}

// cargo test onewire::test -- --exact
#[test]
fn test() {
    let address = OWAddress(0x1E_000000000000_28);
    // assert_eq!(address.family_code(), 0x28);
    // assert_eq!(address.serial_number(), 0x00);
    // assert_eq!(address.crc(), 0x1E);
}
