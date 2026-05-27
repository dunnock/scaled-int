use crate::udecimal64::UDecimal64;
use crate::ParseError;

pub(crate) fn parse<const S: u32>(s: &str) -> Result<UDecimal64<S>, ParseError> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return Err(ParseError::Empty);
    }

    if bytes[0] == b'-' || bytes[0] == b'+' {
        return Err(ParseError::InvalidChar { byte: bytes[0], pos: 0 });
    }

    let mut acc: u64 = 0;
    let mut has_digits = false;
    let mut i = 0usize;

    while i < bytes.len() && bytes[i] != b'.' {
        let d = bytes[i].wrapping_sub(b'0');
        if d > 9 {
            return Err(ParseError::InvalidChar { byte: bytes[i], pos: i });
        }
        acc = acc
            .checked_mul(10)
            .and_then(|v| v.checked_add(d as u64))
            .ok_or(ParseError::Overflow)?;
        has_digits = true;
        i += 1;
    }

    let mut frac_digits = 0u32;
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() {
            let d = bytes[i].wrapping_sub(b'0');
            if d > 9 {
                return Err(ParseError::InvalidChar { byte: bytes[i], pos: i });
            }
            has_digits = true;
            if frac_digits < S {
                acc = acc
                    .checked_mul(10)
                    .and_then(|v| v.checked_add(d as u64))
                    .ok_or(ParseError::Overflow)?;
                frac_digits += 1;
            }
            i += 1;
        }
    }

    if !has_digits {
        return Err(ParseError::Empty);
    }

    for _ in frac_digits..S {
        acc = acc.checked_mul(10).ok_or(ParseError::Overflow)?;
    }

    Ok(UDecimal64::from_raw(acc))
}
