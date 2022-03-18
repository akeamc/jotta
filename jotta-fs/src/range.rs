//! Ranges of bytes.
use std::{fmt::Display, ops::RangeBounds};

use reqwest::header::HeaderValue;

/// Simplified abstraction of the `Range` header value in the sense that
/// only one range is allowed.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[allow(clippy::module_name_repetitions)]
pub struct ByteRange {
    start: Option<u64>,
    end: Option<u64>,
}

#[allow(clippy::len_without_is_empty)]
impl ByteRange {
    /// Attempt to create a new range.
    /// 
    /// # Errors
    /// 
    /// Fails if the range is reversed (end comes before start).
    pub fn try_new(start: Option<u64>, end: Option<u64>) -> Result<Self, InvalidRange> {
        if start.unwrap_or(0) > end.unwrap_or(u64::MAX) {
            Err(InvalidRange::Reversed)
        } else {
            Ok(Self { start, end })
        }
    }

    /// Total length of the range.
    #[must_use]
    pub fn len(&self) -> Option<u64> {
        self.end.map(|end| end + 1 - self.start.unwrap_or(0))
    }

    /// Start of the range.
    #[must_use]
    pub fn start(&self) -> u64 {
        self.start.unwrap_or(0)
    }

    /// End of the range.
    #[must_use]
    pub fn end(&self) -> Option<u64> {
        self.end
    }

    /// Construct a full range.
    ///
    /// ```
    /// use jotta_fs::range::ByteRange;
    ///
    /// assert_eq!(ByteRange::full().to_string(), "bytes=0-");
    /// ```
    #[must_use]
    pub fn full() -> Self {
        Self {
            start: None,
            end: None,
        }
    }

    /// Is the range completely unbounded?
    /// 
    /// ```
    /// use jotta_fs::range::ByteRange;
    /// 
    /// assert!(ByteRange::try_new(Some(0), None).unwrap().is_full());
    /// assert!(ByteRange::full()ca.is_full());
    /// assert!(!ByteRange::try_new(Some(10), None).unwrap().is_full());
    /// assert!(!ByteRange::try_new(None, Some(69)).unwrap().is_full());
    /// ```
    #[must_use]
    pub fn is_full(&self) -> bool {
      self.start() == 0 && self.end().is_none()
    }

    /// Convert a standard [`std::ops::Range`] to [`ByteRange`]:
    ///
    /// ```
    /// use jotta_fs::range::ByteRange;
    ///
    /// assert_eq!(ByteRange::try_from_bounds(..5).unwrap().to_string(), "bytes=0-4");
    /// assert_eq!(ByteRange::try_from_bounds(..).unwrap().to_string(), "bytes=0-");
    /// assert_eq!(ByteRange::try_from_bounds(3..=4).unwrap().to_string(), "bytes=3-4");
    /// assert!(ByteRange::try_from_bounds(10..7).is_err()); // reversed
    /// ```
    ///
    /// # Errors
    ///
    /// This function returns an `Error` if the range is reversed, i.e. `start` comes after `end`.
    pub fn try_from_bounds(bounds: impl RangeBounds<u64>) -> Result<Self, InvalidRange> {
        use std::ops::Bound::{Excluded, Included, Unbounded};

        let start = match bounds.start_bound() {
            Included(i) => Some(*i),
            Excluded(e) => Some(e + 1),
            Unbounded => None,
        };

        let end = match bounds.end_bound() {
            Included(i) => Some(*i),
            Excluded(e) => Some(e - 1),
            Unbounded => None,
        };

        Self::try_new(start, end)
    }

    /// Parse the content of a [`Range` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Range).
    ///
    /// ```
    /// use jotta_fs::range::ByteRange;
    ///
    /// assert_eq!(
    ///     ByteRange::parse_http("bytes=0-99, 200-").unwrap(),
    ///     vec![
    ///         ByteRange::try_from_bounds(0..100).unwrap(),
    ///         ByteRange::try_from_bounds(200..).unwrap(),
    ///     ],
    /// );
    /// ```
    pub fn parse_http(header: &str) -> Result<Vec<Self>, HttpRangeParseError> {
        const PREFIX: &str = "bytes=";

        if header.is_empty() {
            return Ok(Vec::new());
        }

        if !header.starts_with(PREFIX) {
            return Err(HttpRangeParseError);
        }

        let ranges = header[PREFIX.len()..]
            .split(|c| c == ',')
            .filter_map(|ra| {
                let ra = ra.trim();

                if ra.is_empty() {
                    return None;
                }

                let mut bounds = ra.splitn(2, '-');

                let start: Option<u64> = bounds.next()?.parse().ok();
                let end: Option<u64> = bounds.next()?.parse().ok();

                if start.is_none() {
                    if let Some(_end) = end {
                        todo!("suffix not implemented");
                    }
                }

                Self::try_new(start, end).ok()
            })
            .collect::<Vec<_>>();

        Ok(ranges)
    }
}

/// HTTP range parse error.
#[derive(Debug, PartialEq)]
pub struct HttpRangeParseError;

/// Invalid range error.
#[derive(Debug, thiserror::Error)]
#[allow(clippy::module_name_repetitions)]
pub enum InvalidRange {
    /// Range is reversed.
    #[error("range is wrong direction")]
    Reversed,
}

impl Display for ByteRange {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "bytes={}-", self.start())?;

        if let Some(end) = self.end() {
            write!(f, "{}", end)?;
        }

        Ok(())
    }
}

impl From<ByteRange> for HeaderValue {
    fn from(value: ByteRange) -> Self {
        HeaderValue::from_str(&value.to_string()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::ByteRange;

    #[test]
    fn http_byte_range_parse() {
        let cases = vec![(
            "bytes=0-100, 200-",
            Ok(vec![
                ByteRange {
                    start: Some(0),
                    end: Some(100),
                },
                ByteRange {
                    start: Some(200),
                    end: None,
                },
            ]),
        )];

        for (header, expected) in cases {
            assert_eq!(ByteRange::parse_http(header), expected);
        }
    }
}
