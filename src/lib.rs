pub use self::error::{Error, Result};

use crate::scratchpad::{ConfigurationRegister, Resolution, Scratchpad, temperature};
use esp_idf_svc::hal::{
    delay::Delay,
    gpio::IOPin,
    onewire::{DeviceSearch, OWAddress, OWCommand, OWDriver},
    peripheral::Peripheral,
    rmt::RmtChannel,
};
use log::debug;
use std::{mem::transmute, thread, time::Duration};

/// The ds18b20 family code
pub const FAMILY_CODE: u8 = 0x28;
/// Max conversion time, up to 750 ms.
const CONVERSION_TIME_NS: u64 = 750_000_000;

const HIGH: i8 = 30;
const LOW: i8 = 19;
const RESOLUTION: Resolution = Resolution::Twelve;

/// The ds18b20 driver for esp32
pub struct Ds18b20Driver<'a> {
    pub driver: OWDriver<'a>,
}

impl<'a> Ds18b20Driver<'a> {
    pub fn new(
        pin: impl Peripheral<P = impl IOPin> + 'a,
        channel: impl Peripheral<P = impl RmtChannel> + 'a,
    ) -> Result<Self> {
        let driver: OWDriver = OWDriver::new(pin, channel)?;
        // let delay = Delay::new_default();
        Ok(Self { driver })
    }

    /// Receive temperature
    pub fn temperature(&mut self, address: &OWAddress) -> Result<f32> {
        self.initialization()?
            .match_rom(address)?
            .convert_temperature()?;
        let scratchpad = self
            .initialization()?
            .match_rom(address)?
            .read_scratchpad()?;
        Ok(scratchpad.temperature)
    }

    /// Start a search for devices attached to the OneWire bus
    pub fn search(&mut self) -> Result<impl Iterator<Item = Result<OWAddress>>> {
        Ok(self.driver.search()?.map(|address| {
            let address = address?;
            let family_code = address.family_code();
            if family_code != FAMILY_CODE {
                return Err(Error::FamilyCode(family_code));
            }
            Ok(address)
        }))
    }

    // pub fn device(&mut self) -> Result<OWAddress> {
    //     let search = self.search()?;
    //     let address = search.next().ok_or(Error::DeviceNotFound)?;
    //     Ok(address)
    // }
    pub fn initialization(&mut self) -> Result<Rom<&mut Self>> {
        self.driver.reset()?;
        Ok(Rom(self))
    }
}

pub struct Rom<T>(T);

/// ROM function commands
impl<'a, 'b> Rom<&'a mut Ds18b20Driver<'b>> {
    /// Read ROM command
    ///
    /// This command allows the bus master to read the DS18B20’s 8-bit family
    /// code, unique 48-bit serial number, and 8-bit CRC. This command can only
    /// be used if there is a single DS18B20 on the bus. If more than one slave
    /// is present on the bus, a data collision will occur when all slaves try
    /// to transmit at the same time (open drain will produce a wired AND
    /// result).
    pub fn read_rom(self) -> Result<OWAddress> {
        self.0.driver.write(&[OWCommand::ReadRom as _])?;
        let mut buffer = [0u8; 8];
        self.0.driver.read(&mut buffer)?;
        crc8::check(&buffer)?;
        let address = u64::from_le_bytes(buffer);
        // TODO
        // Ok(OWAddress(address))
        Ok(unsafe { transmute(address) })
    }

    /// Match ROM command
    ///
    /// The match ROM command, followed by a 64-bit ROM sequence, allows the bus
    /// master to address a specific DS18B20 on a multidrop bus. Only the
    /// DS18B20 that exactly matches the 64-bit ROM sequence will respond to the
    /// following memory function command. All slaves that do not match the
    /// 64-bit ROM sequence will wait for a reset pulse. This command can be
    /// used with a single or multiple devices on the bus.
    pub fn match_rom(self, address: &OWAddress) -> Result<Ram<&'a mut Ds18b20Driver<'b>>> {
        let mut buffer = [0; 9];
        buffer[0] = OWCommand::MatchRom as _;
        buffer[1..9].copy_from_slice(&address.address().to_le_bytes());
        self.0.driver.write(&buffer)?;
        Ok(Ram(self.0))
    }

    /// Skip ROM command
    ///
    /// This command can save time in a single drop bus system by allowing the
    /// bus master to access the memory functions without providing the 64-bit
    /// ROM code. If more than one slave is present on the bus and a Read
    /// command is issued following the Skip ROM command, data collision will
    /// occur on the bus as multiple slaves transmit simultaneously (open drain
    /// pulldowns will produce a wired AND result).
    pub fn skip_rom(self) -> Result<Ram<&'a mut Ds18b20Driver<'b>>> {
        self.0.driver.write(&[OWCommand::SkipRom as _])?;
        Ok(Ram(self.0))
    }

    // /// Search ROM command
    // ///
    // /// When a system is initially brought up, the bus master might not know the
    // /// number of devices on the 1-Wire bus or their 64-bit ROM codes. The
    // /// search ROM command allows the bus master to use a process of elimination
    // /// to identify the 64-bit ROM codes of all slave devices on the bus.
    // pub fn search_rom(self) -> Result<DeviceSearch<'a, 'a>> {
    //     Ok(self.0.driver.search()?)
    // }

    /// Search alarm command
    ///
    /// When a system is initially brought up, the bus master might not know the
    /// number of devices on the 1-Wire bus or their 64-bit ROM codes. The
    /// search ROM command allows the bus master to use a process of elimination
    /// to identify the 64-bit ROM codes of all slave devices on the bus.
    pub fn search_alarm(self) -> Result<()> {
        todo!()
    }
}

/// RAM commands
pub struct Ram<T>(T);

/// RAM commands
impl<'a> Ram<&mut Ds18b20Driver<'a>> {
    /// Reads the entire scratchpad including the CRC byte.
    pub fn read_scratchpad(self) -> Result<Scratchpad> {
        self.0.driver.write(&[Command::ReadScratchpad as _])?;
        let mut buffer = [0u8; 9];
        self.0.driver.read(&mut buffer)?;
        crc8::check(&buffer)?;
        let configuration_register = ConfigurationRegister::try_from(buffer[4])?;
        Ok(Scratchpad {
            temperature: temperature(buffer[1], buffer[0], configuration_register.resolution),
            alarm_high_trigger_register: buffer[2] as _,
            alarm_low_trigger_register: buffer[3] as _,
            configuration_register,
            crc: buffer[8],
        })
    }

    /// Writes TH, TL, and configuration register data into scratchpad.
    pub fn write_scratchpad(self, scratchpad: &Scratchpad) -> Result<()> {
        self.0.driver.write(&[Command::WriteScratchpad as _])?;
        let buffer = [
            scratchpad.alarm_high_trigger_register as _,
            scratchpad.alarm_low_trigger_register as _,
            scratchpad.configuration_register.into(),
        ];
        Ok(self.0.driver.write(&buffer)?)
    }

    /// Load TH, TL, and configuration register data from the scratchpad to
    /// EEPROM.
    pub fn load_scratchpad(self) -> Result<()> {
        todo!()
    }

    /// Save TH, TL, and configuration register data from EEPROM to the
    /// scratchpad.
    pub fn save_scratchpad(self) -> Result<()> {
        todo!()
    }

    /// This command begins a temperature conversion. No further data is
    /// required. The temperature conversion will be performed and then the
    /// DS18B20 will remain idle. If the bus master issues read time slots
    /// following this command, the DS18B20 will output 0 on the bus as long as
    /// it is busy making a temperature conversion; it will return a 1 when the
    /// temperature conversion is complete. If parasite-powered, the bus master
    /// has to enable a strong pullup for a period greater than tconv
    /// immediately after issuing this command.
    ///
    /// You should wait for the measurement to finish before reading the
    /// measurement. The amount of time you need to wait depends on the current
    /// resolution configuration
    pub fn convert_temperature(self) -> Result<()> {
        self.0.driver.write(&[Command::ConvertTemperature as _])?;
        // delay proper time for temp conversion, assume max resolution
        // (12-bits)
        thread::sleep(Duration::from_nanos(CONVERSION_TIME_NS));
        Ok(())
    }

    /// Signals the mode of DS18B20 power supply to the master.
    pub fn read_power_supply(self) -> Result<()> {
        todo!()
    }
}

#[allow(dead_code)]
#[repr(u8)]
enum Command {
    WriteScratchpad = 0x4E,
    ReadScratchpad = 0xBE,
    CopyScratchpad = 0x48,
    ConvertTemperature = 0x44,
    RecallE2Memory = 0xB8,
    ReadPowerSupply = 0xB4,
}

pub mod crc8;
pub mod error;
pub mod scratchpad;
