//! Zero-I/O value objects, the two message aggregates and the one typed error
//! of this port (`ADR-0010` §1).
//!
//! Nothing here names a socket, a TLS type or an operating-system handle: the
//! domain describes *what an HTTP exchange is*, and `infrastructure/` is the
//! only layer that knows how one is performed.

pub mod authority;
pub mod body;
pub mod defect;
pub mod error;
pub mod header_map;
pub mod media_type;
pub mod method;
pub mod phase;
pub mod request;
pub mod response;
pub mod scheme;
pub mod status;
pub mod target;
pub mod url;
