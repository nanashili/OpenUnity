use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// A validated 32-hexadecimal-digit GUID, displayed in lowercase.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct Guid([u8; 16]);

/// A GUID was not exactly 32 ASCII hexadecimal digits.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct InvalidGuid;

impl fmt::Display for InvalidGuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GUID must contain exactly 32 hexadecimal digits")
    }
}

impl Error for InvalidGuid {}

impl FromStr for Guid {
    type Err = InvalidGuid;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        if value.len() != 32 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(InvalidGuid);
        }
        let mut bytes = [0; 16];
        for (output, pair) in bytes.iter_mut().zip(value.as_bytes().chunks_exact(2)) {
            let digit = |byte: u8| match byte {
                b'0'..=b'9' => byte - b'0',
                b'a'..=b'f' => byte - b'a' + 10,
                b'A'..=b'F' => byte - b'A' + 10,
                _ => unreachable!("validated hexadecimal digit"),
            };
            *output = (digit(pair[0]) << 4) | digit(pair[1]);
        }
        Ok(Self(bytes))
    }
}

impl fmt::Display for Guid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for byte in self.0 {
            write!(f, "{byte:02x}")?;
        }
        Ok(())
    }
}
