use crate::{Decimal64, ParseError};

pub(crate) fn parse<const S: u32>(_s: &str) -> Result<Decimal64<S>, ParseError> {
    Err(ParseError::Empty)
}
