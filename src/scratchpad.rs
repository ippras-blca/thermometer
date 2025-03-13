use crate::{CONVERSION_TIME_NS, error::Error};

pub(crate) const NINE: u8 = 0b00011111;
pub(crate) const TEN: u8 = 0b00111111;
pub(crate) const ELEVEN: u8 = 0b01011111;
pub(crate) const TWELVE: u8 = 0b01111111;

/// Scratchpad
#[derive(Clone, Copy, Debug, Default)]
pub struct Scratchpad {
    pub temperature: f32,
    /// Alarm high trigger register (TH)
    pub alarm_high_trigger_register: i8,
    /// Alarm low trigger register (TL)
    pub alarm_low_trigger_register: i8,
    /// Configuration register
    pub configuration_register: ConfigurationRegister,
    pub crc: u8,
}

/// Configuration register
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ConfigurationRegister {
    pub resolution: Resolution,
}

impl TryFrom<u8> for ConfigurationRegister {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            NINE => Ok(Self {
                resolution: Resolution::Nine,
            }),
            TEN => Ok(Self {
                resolution: Resolution::Ten,
            }),
            ELEVEN => Ok(Self {
                resolution: Resolution::Eleven,
            }),
            TWELVE => Ok(Self {
                resolution: Resolution::Twelve,
            }),
            configuration_register => Err(Error::ConfigurationRegister {
                configuration_register,
            }),
        }
    }
}

impl From<ConfigurationRegister> for u8 {
    fn from(value: ConfigurationRegister) -> Self {
        match value.resolution {
            Resolution::Nine => NINE,
            Resolution::Ten => TEN,
            Resolution::Eleven => ELEVEN,
            Resolution::Twelve => TWELVE,
        }
    }
}

/// Temperature resolution: 9, 10, 11 or 12 bits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Resolution {
    /// 9-bit, equates to a temperature resolution of 0.5°C
    Nine,
    /// 10-bit, equates to a temperature resolution of 0.25°C
    Ten,
    /// 11-bit, equates to a temperature resolution of 0.125°C
    Eleven,
    /// 12-bit, equates to a temperature resolution of 0.0625°C
    #[default]
    Twelve,
}

impl Resolution {
    /// Conversion time (ns)
    pub fn conversion_time(&self) -> u32 {
        (match self {
            Resolution::Nine => CONVERSION_TIME_NS / 8,
            Resolution::Ten => CONVERSION_TIME_NS / 4,
            Resolution::Eleven => CONVERSION_TIME_NS / 2,
            Resolution::Twelve => CONVERSION_TIME_NS,
        }) as _
    }
}

pub fn temperature(msb: u8, lsb: u8, resolution: Resolution) -> f32 {
    i16::from_be_bytes([msb, lsb]) as f32 / 16.0
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn configuration_register() {
        assert_eq!(
            Err(Error::ConfigurationRegister {
                configuration_register: 0b0_00_11110
            }),
            ConfigurationRegister::try_from(0b0_00_11110),
        );
        assert_eq!(
            Err(Error::ConfigurationRegister {
                configuration_register: 0b0_00_11101
            }),
            ConfigurationRegister::try_from(0b0_00_11101),
        );
        assert_eq!(
            Err(Error::ConfigurationRegister {
                configuration_register: 0b0_00_11011
            }),
            ConfigurationRegister::try_from(0b0_00_11011),
        );
        assert_eq!(
            Err(Error::ConfigurationRegister {
                configuration_register: 0b0_00_10111
            }),
            ConfigurationRegister::try_from(0b0_00_10111),
        );
        assert_eq!(
            Err(Error::ConfigurationRegister {
                configuration_register: 0b0_00_01111
            }),
            ConfigurationRegister::try_from(0b0_00_01111),
        );
        assert_eq!(
            Err(Error::ConfigurationRegister {
                configuration_register: 0b1_00_11111
            }),
            ConfigurationRegister::try_from(0b1_00_11111),
        );
    }

    #[test]
    fn temperature() {
        use super::temperature;

        assert_eq!(125.0, temperature(0x07, 0xD0, Default::default()));
        assert_eq!(85.0, temperature(0x05, 0x50, Default::default()));
        assert_eq!(25.0625, temperature(0x01, 0x91, Default::default()));
        assert_eq!(10.125, temperature(0x00, 0xA2, Default::default()));
        assert_eq!(0.5, temperature(0x00, 0x08, Default::default()));
        assert_eq!(0.0, temperature(0x00, 0x00, Default::default()));
        assert_eq!(-0.5, temperature(0xFF, 0xF8, Default::default()));
        assert_eq!(-10.125, temperature(0xFF, 0x5E, Default::default()));
        assert_eq!(-25.0625, temperature(0xFE, 0x6F, Default::default()));
        assert_eq!(-55.0, temperature(0xFC, 0x90, Default::default()));
    }
}
