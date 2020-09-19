//! x-amz-date

#![allow(dead_code)]

/// x-amz-date
#[allow(missing_copy_implementations)]
#[derive(Debug)]
pub struct AmzDate {
    /// year
    year: u32,
    /// month
    month: u32,
    /// day
    day: u32,
    /// hour
    hour: u32,
    /// minute
    minute: u32,
    /// second
    second: u32,
}

/// `ParseAmzDateError`
#[allow(missing_copy_implementations)]
#[derive(Debug, thiserror::Error)]
#[error("ParseAmzDateError")]
pub struct ParseAmzDateError {
    /// private place holder
    _priv: (),
}

impl AmzDate {
    /// x-amz-date time format
    const X_AMZ_DATE_FORMAT: &'static str = "%Y%m%dT%H%M%SZ";

    /// Parses `AmzDate` from header
    /// # Errors
    /// Returns an error if the header is invalid
    pub fn from_header_str(header: &str) -> Result<Self, ParseAmzDateError> {
        /// nom parser
        fn parse(input: &str) -> nom::IResult<&str, [&str; 6]> {
            use nom::{
                bytes::complete::{tag, take},
                combinator::{all_consuming, verify},
                sequence::tuple,
            };

            let parser = verify(
                all_consuming(tuple((
                    take(4_usize),
                    take(2_usize),
                    take(2_usize),
                    tag("T"),
                    take(2_usize),
                    take(2_usize),
                    take(2_usize),
                    tag("Z"),
                ))),
                |(year_str, month_str, day_str, _, hour_str, minute_str, second_str, _)| {
                    [
                        year_str, month_str, day_str, hour_str, minute_str, second_str,
                    ]
                    .iter()
                    .copied()
                    .all(|s: &&str| s.as_bytes().iter().all(u8::is_ascii_digit))
                },
            );

            let (_, (year_str, month_str, day_str, _, hour_str, minute_str, second_str, _)) =
                parser(input)?;

            Ok((
                input,
                [
                    year_str, month_str, day_str, hour_str, minute_str, second_str,
                ],
            ))
        }

        /// parse u32
        fn to_u32(input: &str) -> Result<u32, ParseAmzDateError> {
            match input.parse::<u32>() {
                Ok(x) => Ok(x),
                Err(_) => Err(ParseAmzDateError { _priv: () }),
            }
        }

        match parse(header) {
            Err(_) => Err(ParseAmzDateError { _priv: () }),
            Ok((_, [year_str, month_str, day_str, hour_str, minute_str, second_str])) => Ok(Self {
                year: to_u32(year_str)?,
                month: to_u32(month_str)?,
                day: to_u32(day_str)?,
                hour: to_u32(hour_str)?,
                minute: to_u32(minute_str)?,
                second: to_u32(second_str)?,
            }),
        }
    }

    /// `YYYYMMDD'T'HHMMSS'Z'`
    #[must_use]
    pub fn to_iso8601(&self) -> String {
        format!(
            "{:04}{:02}{:02}T{:02}{:02}{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// `YYYYMMDD`
    #[must_use]
    pub fn to_date(&self) -> String {
        format!("{:04}{:02}{:02}", self.year, self.month, self.day,)
    }
}
