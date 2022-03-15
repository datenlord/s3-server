//! HTTP Range header

/// HTTP Range header
///
/// See <https://www.w3.org/Protocols/rfc2616/rfc2616-sec14.html#sec14.35>
#[allow(clippy::exhaustive_enums, missing_copy_implementations)]
#[derive(Debug, Clone)]
pub enum Range {
    /// Normal byte range
    Normal {
        /// first
        first: u64,
        /// last
        last: Option<u64>,
    },
    /// Suffix byte range
    Suffix {
        /// last
        last: u64,
    },
}

/// `ParseRangeError`
#[allow(missing_copy_implementations)] // Why? See `crate::path::ParseS3PathError`.
#[derive(Debug, thiserror::Error)]
#[error("ParseRangeError")]
pub struct ParseRangeError {
    /// private place holder
    _priv: (),
}

impl Range {
    /// Parses `Range` from header
    /// # Errors
    /// Returns an error if the header is invalid
    pub fn from_header_str(header: &str) -> Result<Self, ParseRangeError> {
        /// nom parser
        fn parse(input: &str) -> nom::IResult<&str, Range> {
            use nom::{
                branch::alt,
                bytes::complete::tag,
                character::complete::digit1,
                combinator::{all_consuming, map, map_res, opt},
                sequence::tuple,
            };

            let normal_parser = map_res(
                tuple((
                    map_res(digit1, str::parse::<u64>),
                    tag("-"),
                    opt(map_res(digit1, str::parse::<u64>)),
                )),
                |ss: (u64, &str, Option<u64>)| {
                    if let (first, Some(last)) = (ss.0, ss.2) {
                        if first > last {
                            return Err(ParseRangeError { _priv: () });
                        }
                    }
                    Ok(Range::Normal {
                        first: ss.0,
                        last: ss.2,
                    })
                },
            );

            let suffix_parser = map(
                tuple((tag("-"), map_res(digit1, str::parse::<u64>))),
                |ss: (&str, u64)| Range::Suffix { last: ss.1 },
            );

            let mut parser =
                all_consuming(tuple((tag("bytes="), alt((normal_parser, suffix_parser)))));

            let (input, (_, ans)) = parser(input)?;

            Ok((input, ans))
        }

        match parse(header) {
            Err(_) => Err(ParseRangeError { _priv: () }),
            Ok((_, ans)) => Ok(ans),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_range() {
        {
            let src = "bytes=0-499";
            let result = Range::from_header_str(src);
            assert!(matches!(
                result.unwrap(),
                Range::Normal {
                    first: 0,
                    last: Some(499)
                }
            ));
        }
        {
            let src = "bytes=0-499;";
            let result = Range::from_header_str(src);
            assert!(result.is_err());
        }
        {
            let src = "bytes=9500-";
            let result = Range::from_header_str(src);
            assert!(matches!(
                result.unwrap(),
                Range::Normal {
                    first: 9500,
                    last: None
                }
            ));
        }
        {
            let src = "bytes=9500-0-";
            let result = Range::from_header_str(src);
            assert!(result.is_err());
        }
        {
            let src = "bytes=-500";
            let result = Range::from_header_str(src);
            assert!(matches!(result.unwrap(), Range::Suffix { last: 500 }));
        }
        {
            let src = "bytes=-500 ";
            let result = Range::from_header_str(src);
            assert!(result.is_err());
        }
        {
            let src = "bytes=-1000000000000000000000000";
            let result = Range::from_header_str(src);
            assert!(result.is_err());
        }
    }
}
