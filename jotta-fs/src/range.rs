//! Ranges of bytes.
use std::{
    fmt::Debug,
    ops::{Range, RangeFrom, RangeFull, RangeInclusive, RangeTo, RangeToInclusive},
};

use reqwest::header::HeaderValue;

/// An optionally half-open range of bytes.
#[allow(clippy::module_name_repetitions)]
pub trait ByteRange: Debug {
    /// The first byte of the range (inclusive).
    fn start(&self) -> u64;

    /// The last byte of the range (inclusive).
    fn end(&self) -> Option<u64> {
        self.len().map(|len| len + self.start() - 1)
    }

    /// Get the length.
    ///
    /// ```
    /// use jotta_fs::range::{ByteRange, OpenByteRange, ClosedByteRange};
    ///
    /// assert_eq!(ByteRange::len(&ClosedByteRange::new_to_including(3)), Some(4));
    /// assert_eq!(ByteRange::len(&ClosedByteRange::try_from_bounds(3, 7).unwrap()), Some(5));
    /// assert_eq!(OpenByteRange::full().len(), None);
    /// ```
    fn len(&self) -> Option<u64>;

    /// Is the range empty?
    ///
    /// ```
    /// use jotta_fs::range::{ByteRange, OpenByteRange, ClosedByteRange};
    ///
    /// assert!(!ClosedByteRange::new_to_including(3).is_empty());
    /// assert!(!OpenByteRange::new(10).is_empty());
    /// assert!(!OpenByteRange::full().is_empty());
    /// assert!(ClosedByteRange::new(5, 0).is_empty());
    /// ```
    fn is_empty(&self) -> bool {
        self.len().map_or(false, |len| len == 0)
    }

    /// Format a single "segment" of a HTTP `Range` header.
    ///
    /// ```
    /// use jotta_fs::range::{ByteRange, OpenByteRange, ClosedByteRange};
    ///
    /// assert_eq!(ClosedByteRange::try_from_bounds(5, 50).unwrap().to_http_range(), "5-50");
    ///
    /// assert_eq!(OpenByteRange::new(100).to_http_range(), "100-");
    /// assert_eq!(OpenByteRange::full().to_http_range(), "0-");
    /// ```
    fn to_http_range(&self) -> String {
        format!(
            "{}-{}",
            self.start(),
            self.end().map(|n| n.to_string()).unwrap_or_default(),
        )
    }

    /// Format a [HTTP `Range` header](https://developer.mozilla.org/en-US/docs/Web/HTTP/Headers/Range).
    ///
    fn to_http(&self) -> HeaderValue {
        let s = format!("bytes={}", self.to_http_range());
        HeaderValue::from_str(&s).unwrap()
    }
}

impl ByteRange for OpenByteRange {
    fn start(&self) -> u64 {
        self.start
    }

    fn len(&self) -> Option<u64> {
        None
    }
}

/// An open byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(clippy::module_name_repetitions)]
pub struct OpenByteRange {
    start: u64,
}

impl OpenByteRange {
    /// Construct a new half-open byte range.
    #[must_use]
    pub fn new(start: u64) -> Self {
        Self { start }
    }

    /// Construct a full byte range.
    #[must_use]
    pub fn full() -> Self {
        Self::new(0)
    }
}

impl From<RangeFrom<u64>> for OpenByteRange {
    fn from(r: RangeFrom<u64>) -> Self {
        Self { start: r.start }
    }
}

impl From<RangeFull> for OpenByteRange {
    fn from(_: RangeFull) -> Self {
        Self { start: 0 }
    }
}

/// A closed byte range.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[allow(clippy::module_name_repetitions)]
pub struct ClosedByteRange {
    start: u64,
    len: u64,
}

/// Invalid range.
#[derive(Debug, thiserror::Error)]
pub enum InvalidRangeError {
    /// Byte ranges must not be backwards.
    #[error("range is backwards")]
    Backwards,
}

impl ClosedByteRange {
    /// Construct a new closed byte range.
    #[must_use]
    pub fn new(start: u64, len: u64) -> Self {
        Self { start, len }
    }

    /// Attempt to construct a new closed byte range from some specified bounds.
    ///
    /// ```
    /// use jotta_fs::range::ClosedByteRange;
    ///
    /// let range = ClosedByteRange::try_from_bounds(3, 8).unwrap();
    ///
    /// assert_eq!(range.start(), 3);
    /// assert_eq!(range.end(), 8);
    /// assert_eq!(range.len(), 6);
    /// ```
    ///
    /// # Errors
    ///
    /// ```
    /// # use jotta_fs::range::ClosedByteRange;
    /// assert!(ClosedByteRange::try_from_bounds(100, 0).is_err()); // reversed
    /// ```
    pub fn try_from_bounds(first: u64, last: u64) -> Result<Self, InvalidRangeError> {
        if first > last {
            Err(InvalidRangeError::Backwards)
        } else {
            Ok(Self::new(first, last - first + 1))
        }
    }

    /// Construct a new closed byte range with a specified last byte.
    /// First byte will be 0.
    ///
    /// ```
    /// use jotta_fs::range::ClosedByteRange;
    ///
    /// assert_eq!(ClosedByteRange::new_to_including(10), ClosedByteRange::try_from_bounds(0, 10).unwrap())
    /// ```
    #[must_use]
    pub fn new_to_including(end: u64) -> Self {
        Self::new(0, end + 1)
    }

    /// How many bytes this range includes.
    #[must_use]
    pub fn len(&self) -> u64 {
        self.len
    }

    /// Is the range empty?
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Start of the range.
    #[must_use]
    pub fn start(&self) -> u64 {
        self.start
    }

    /// Get the end byte.
    ///
    /// ```
    /// use jotta_fs::range::ClosedByteRange;
    ///
    /// assert_eq!(ClosedByteRange::try_from_bounds(5, 20).unwrap().end(), 20);
    /// ```
    #[must_use]
    pub fn end(&self) -> u64 {
        self.start + self.len - 1
    }
}

impl ByteRange for ClosedByteRange {
    fn start(&self) -> u64 {
        self.start
    }

    fn len(&self) -> Option<u64> {
        Some(self.len)
    }
}

impl TryFrom<Range<u64>> for ClosedByteRange {
    type Error = InvalidRangeError;

    fn try_from(r: Range<u64>) -> Result<Self, Self::Error> {
        Self::try_from_bounds(r.start, r.end - 1)
    }
}

impl TryFrom<RangeInclusive<u64>> for ClosedByteRange {
    type Error = InvalidRangeError;

    fn try_from(r: RangeInclusive<u64>) -> Result<Self, Self::Error> {
        Self::try_from_bounds(*r.start(), *r.end())
    }
}

impl From<RangeTo<u64>> for ClosedByteRange {
    fn from(r: RangeTo<u64>) -> Self {
        Self::new_to_including(r.end - 1)
    }
}

impl From<RangeToInclusive<u64>> for ClosedByteRange {
    fn from(r: RangeToInclusive<u64>) -> Self {
        Self::new_to_including(r.end)
    }
}
