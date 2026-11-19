// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Dates and date-times as their standard lexical forms (table projection
//! plan, T2).
//!
//! A value keeps the string it was disclosed as. What is accepted is the
//! forms both standards agree on: a date is RFC 3339's `full-date`, which is
//! an `xsd:date`; a date-time is RFC 3339's `date-time` written as
//! `xsd:dateTime` allows it (upper-case `T` and `Z`, an offset within
//! fourteen hours, no leap second). A date-time always carries its offset,
//! so it names one instant.

use std::cmp::Ordering;

/// A parsed `xsd:date`: year, month, day.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

/// A parsed `xsd:dateTime`, reduced to the instant it names.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Instant {
    /// Whole seconds since 1970-01-01T00:00:00Z.
    pub seconds: i64,
    /// Fractional-second digits with trailing zeros removed, so that digit
    /// strings compare in the same order as the fractions they write.
    pub fraction: String,
}

impl Ord for Instant {
    fn cmp(&self, other: &Self) -> Ordering {
        self.seconds
            .cmp(&other.seconds)
            .then_with(|| self.fraction.cmp(&other.fraction))
    }
}

impl PartialOrd for Instant {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Read `YYYY-MM-DD`.
///
/// # Errors
/// Says why the text is not such a date.
pub fn parse_date(text: &str) -> Result<Date, String> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes[4] != b'-' || bytes[7] != b'-' {
        return Err(format!("{text:?} is not a date written YYYY-MM-DD"));
    }
    let year = digits(&bytes[0..4]).ok_or_else(|| format!("{text:?} has no four-digit year"))?;
    let month = digits(&bytes[5..7]).ok_or_else(|| format!("{text:?} has no two-digit month"))?;
    let day = digits(&bytes[8..10]).ok_or_else(|| format!("{text:?} has no two-digit day"))?;
    if !(1..=12).contains(&month) {
        return Err(format!("{text:?} names month {month}"));
    }
    if day == 0 || day > days_in_month(year, month) {
        return Err(format!("{text:?} names a day its month does not have"));
    }
    Ok(Date {
        year: year as u16,
        month: month as u8,
        day: day as u8,
    })
}

/// Read `YYYY-MM-DDTHH:MM:SS[.fraction](Z|+HH:MM|-HH:MM)`.
///
/// # Errors
/// Says why the text is not such a date-time.
pub fn parse_date_time(text: &str) -> Result<Instant, String> {
    let bytes = text.as_bytes();
    let refuse = |what: &str| format!("{text:?} is not a date-time: {what}");
    if bytes.len() < 20 || bytes[10] != b'T' {
        return Err(refuse("expected YYYY-MM-DDTHH:MM:SS and an offset"));
    }
    let date = parse_date(&text[..10]).map_err(|_| refuse("its date part is not a date"))?;
    if bytes[13] != b':' || bytes[16] != b':' {
        return Err(refuse("its time is not written HH:MM:SS"));
    }
    let hour = digits(&bytes[11..13]).ok_or_else(|| refuse("no two-digit hour"))?;
    let minute = digits(&bytes[14..16]).ok_or_else(|| refuse("no two-digit minute"))?;
    let second = digits(&bytes[17..19]).ok_or_else(|| refuse("no two-digit second"))?;
    if hour > 23 || minute > 59 || second > 59 {
        return Err(refuse("its time is out of range"));
    }
    let mut rest = &bytes[19..];
    let mut fraction = "";
    if let Some(after) = rest.strip_prefix(b".") {
        let len = after.iter().take_while(|b| b.is_ascii_digit()).count();
        if len == 0 {
            return Err(refuse("a decimal point with no digits after it"));
        }
        fraction = &text[20..20 + len];
        rest = &after[len..];
    }
    let offset_minutes = match rest {
        b"Z" => 0,
        [sign @ (b'+' | b'-'), h1, h2, b':', m1, m2] => {
            let hours = digits(&[*h1, *h2]).ok_or_else(|| refuse("its offset hour"))?;
            let minutes = digits(&[*m1, *m2]).ok_or_else(|| refuse("its offset minute"))?;
            if minutes > 59 || hours > 14 || (hours == 14 && minutes > 0) {
                return Err(refuse("its offset is beyond fourteen hours"));
            }
            let total = i64::from(hours * 60 + minutes);
            if *sign == b'-' { -total } else { total }
        },
        [] => return Err(refuse("it has no offset, so it names no instant")),
        _ => return Err(refuse("its offset is not Z or +HH:MM or -HH:MM")),
    };
    let days = days_from_civil(i64::from(date.year), date.month.into(), date.day.into());
    let local = days * 86_400 + i64::from(hour * 3600 + minute * 60 + second);
    Ok(Instant {
        seconds: local - offset_minutes * 60,
        fraction: fraction.trim_end_matches('0').to_owned(),
    })
}

fn digits(bytes: &[u8]) -> Option<u32> {
    bytes.iter().try_fold(0u32, |sum, byte| {
        byte.is_ascii_digit()
            .then(|| sum * 10 + u32::from(byte - b'0'))
    })
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        },
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days from 1970-01-01 in the proleptic Gregorian calendar (Howard
/// Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (month + if month > 2 { -3 } else { 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
#[path = "temporal_tests.rs"]
mod tests;
