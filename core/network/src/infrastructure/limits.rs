//! [`WireLimits`] — every ceiling the HTTP/1.1 reader enforces.
//!
//! A hostile server's cheapest attacks are unbounded ones: a header line that
//! never ends, ten million header fields, a body that never stops. Each of them
//! is answered by a number here, and each number produces
//! [`NetworkError::LimitExceeded`] naming the [`WireLimit`] that was hit, so a
//! test asserts the discriminant rather than a message.
//!
//! [`NetworkError::LimitExceeded`]: crate::domain::error::NetworkError::LimitExceeded

use crate::domain::defect::WireLimit;

/// A ceiling measured in bytes. A newtype so a limit is never mistaken for a
/// length (Object Calisthenics rule 3, `ADR-0010:129`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ByteCap(usize);

impl ByteCap {
    /// A ceiling of `bytes` bytes.
    #[must_use]
    pub const fn of_bytes(bytes: usize) -> Self {
        Self(bytes)
    }

    /// The ceiling in bytes.
    #[must_use]
    pub const fn bytes(self) -> usize {
        self.0
    }

    /// Whether `observed` has gone past this ceiling.
    #[must_use]
    pub const fn is_exceeded_by(self, observed: usize) -> bool {
        observed > self.0
    }
}

/// A ceiling measured in field lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct FieldCap(usize);

impl FieldCap {
    /// A ceiling of `fields` field lines.
    #[must_use]
    pub const fn of_fields(fields: usize) -> Self {
        Self(fields)
    }

    /// The ceiling in field lines.
    #[must_use]
    pub const fn fields(self) -> usize {
        self.0
    }

    /// Whether `observed` has gone past this ceiling.
    #[must_use]
    pub const fn is_exceeded_by(self, observed: usize) -> bool {
        observed > self.0
    }
}

/// The complete set of ceilings one exchange is read under.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct WireLimits {
    status_line: ByteCap,
    header_line: ByteCap,
    header_count: FieldCap,
    chunk_line: ByteCap,
    body: ByteCap,
    decoded_body: ByteCap,
}

impl WireLimits {
    /// The defaults every adapter starts from: 8 KiB per line, 100 fields,
    /// 32 MiB of body on the wire, 64 MiB after decompression.
    ///
    /// The last pair is the decompression-bomb ceiling: a 32 MiB `gzip` body
    /// cannot expand past 64 MiB before
    /// [`InflateError::OutputLimitExceeded`](crate::infrastructure::inflate::InflateError::OutputLimitExceeded)
    /// stops it.
    pub const DEFAULT: Self = Self {
        status_line: ByteCap::of_bytes(8 * 1024),
        header_line: ByteCap::of_bytes(8 * 1024),
        header_count: FieldCap::of_fields(100),
        chunk_line: ByteCap::of_bytes(1024),
        body: ByteCap::of_bytes(32 * 1024 * 1024),
        decoded_body: ByteCap::of_bytes(64 * 1024 * 1024),
    };

    /// The status line ceiling.
    #[must_use]
    pub const fn status_line(self) -> ByteCap {
        self.status_line
    }

    /// The per-field-line ceiling.
    #[must_use]
    pub const fn header_line(self) -> ByteCap {
        self.header_line
    }

    /// The field-count ceiling.
    #[must_use]
    pub const fn header_count(self) -> FieldCap {
        self.header_count
    }

    /// The chunk-size-line ceiling.
    #[must_use]
    pub const fn chunk_line(self) -> ByteCap {
        self.chunk_line
    }

    /// The on-the-wire body ceiling.
    #[must_use]
    pub const fn body(self) -> ByteCap {
        self.body
    }

    /// The post-decompression body ceiling.
    #[must_use]
    pub const fn decoded_body(self) -> ByteCap {
        self.decoded_body
    }

    /// The same limits with a different body ceiling — what a test that wants
    /// to prove the ceiling works uses, so it need not build 32 MiB.
    #[must_use]
    pub const fn with_body_cap(mut self, cap: ByteCap) -> Self {
        self.body = cap;
        self
    }

    /// The same limits with a different per-line ceiling.
    #[must_use]
    pub const fn with_header_line_cap(mut self, cap: ByteCap) -> Self {
        self.header_line = cap;
        self
    }

    /// Which [`WireLimit`] a body overrun reports.
    #[must_use]
    pub const fn body_limit_kind() -> WireLimit {
        WireLimit::BodyLength
    }
}

impl Default for WireLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}
