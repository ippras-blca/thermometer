use crate::{
    FAMILY_CODE,
    scratchpad::{ELEVEN, NINE, TEN, TWELVE},
};
use esp_idf_svc::sys::EspError;
use thiserror::Error;

/// Result
pub type Result<T, E = Error> = core::result::Result<T, E>;

/// Error
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
pub enum Error {
    #[error(transparent)]
    Esp(#[from] EspError),
    #[error("device not found")]
    DeviceNotFound,
    #[error("unexpected family code {{ family_code={0}, expected={FAMILY_CODE:x} }}")]
    FamilyCode(u8),
    #[error(
        "unexpected configuration register {{ configuration_register={configuration_register:b}, expected=[{NINE:b}, {TEN:b}, {ELEVEN:b}, {TWELVE:b}] }}"
    )]
    ConfigurationRegister { configuration_register: u8 },
    #[error(transparent)]
    Crc(#[from] CrcError),
}

/// The CRC error
#[derive(Clone, Copy, Debug, Eq, Error, PartialEq)]
#[error("unexpected CRC {{ crc={crc}, expected=0 }}")]
pub struct CrcError {
    pub(crate) crc: u8,
}
