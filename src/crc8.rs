use crate::error::CrcError;

/// Calculates the crc8 of the input data.
pub fn calculate(data: &[u8]) -> u8 {
    calculate_with_initial(0, data)
}

/// Calculates the crc8 of the input data with init value.
///
/// Feedback polynomial: `X^8 + X^5 + X^4 + X^0`
/// LFSR (Galois configuration):
///                                                         v     input bit
/// [7]->[6]->[5]->[4]->(XOR)->[3]->(XOR)->[2]->[1]->[0]->(XOR)-> feedback bit
///  ^                    ^           ^                           feedback mask
pub fn calculate_with_initial(mut crc: u8, data: &[u8]) -> u8 {
    for byte in data {
        crc ^= byte;
        for _ in 0..u8::BITS {
            // feedback bit at each iteration step
            let bit = crc & 0b1;
            crc >>= 1;
            // feedback mask (if feedback bit)
            if bit != 0 {
                crc ^= 0b1000_1100;
            }
        }
    }
    crc
}

/// Checks to see if data (including the crc byte) passes the crc check.
pub fn check(data: &[u8]) -> Result<(), CrcError> {
    match calculate(data) {
        0 => Ok(()),
        crc => Err(CrcError { crc }),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn calculate() {
        use super::calculate;

        assert_eq!(calculate(&[99, 1, 75, 70, 127, 255, 13, 16]), 21);
        assert_eq!(calculate(&[99, 1, 75, 70, 127, 255, 13, 16, 21]), 0);

        assert_eq!(calculate(&[97, 1, 75, 70, 127, 255, 15, 16]), 2);
        assert_eq!(calculate(&[97, 1, 75, 70, 127, 255, 15, 16, 2]), 0);

        assert_eq!(calculate(&[95, 1, 75, 70, 127, 255, 1, 16]), 155);
        assert_eq!(calculate(&[95, 1, 75, 70, 127, 255, 1, 16, 155]), 0);
    }

    #[test]
    fn check() {
        use super::check;

        assert_eq!(
            check(&[99, 1, 75, 70, 127, 255, 13, 16]),
            Err(CrcError { crc: 21 })
        );
        assert!(check(&[99, 1, 75, 70, 127, 255, 13, 16, 21]).is_ok());

        assert_eq!(
            check(&[97, 1, 75, 70, 127, 255, 15, 16]),
            Err(CrcError { crc: 2 })
        );
        assert!(check(&[97, 1, 75, 70, 127, 255, 15, 16, 2]).is_ok());

        assert_eq!(
            check(&[95, 1, 75, 70, 127, 255, 1, 16]),
            Err(CrcError { crc: 155 })
        );
        assert!(check(&[95, 1, 75, 70, 127, 255, 1, 16, 155]).is_ok());
    }
}
