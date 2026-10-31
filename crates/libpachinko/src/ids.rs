//! The module's names and values: who publishes and consumes, which pocket and key
//! a record has, its rank and payload, and the positions the queue hands out.

use std::fmt;

/// Declares a newtype over `String` that names something in the module.
macro_rules! name {
    ($(#[$doc:meta])* $name:ident, $example:literal) => {
        $(#[$doc])*
        ///
        /// ```
        #[doc = concat!("let name = libpachinko::", stringify!($name), "::new(\"", $example, "\");")]
        #[doc = concat!("assert_eq!(name.as_str(), \"", $example, "\");")]
        #[doc = concat!("assert_eq!(name.to_string(), \"", $example, "\");")]
        #[doc = concat!("assert_eq!(name, libpachinko::", stringify!($name), "::from(\"", $example, "\"));")]
        /// ```
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub struct $name(String);

        impl $name {
            /// The name, from any string.
            ///
            /// ```
            #[doc = concat!("let name = libpachinko::", stringify!($name), "::new(String::from(\"", $example, "\"));")]
            #[doc = concat!("assert_eq!(name.as_str(), \"", $example, "\");")]
            /// ```
            #[must_use]
            pub fn new(name: impl Into<String>) -> Self {
                Self(name.into())
            }

            /// The name as text.
            ///
            /// ```
            #[doc = concat!("let name = libpachinko::", stringify!($name), "::new(\"", $example, "\");")]
            #[doc = concat!("assert_eq!(name.as_str().len(), ", stringify!($example), ".len());")]
            /// ```
            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl From<&str> for $name {
            fn from(name: &str) -> Self {
                Self::new(name)
            }
        }

        impl From<String> for $name {
            fn from(name: String) -> Self {
                Self(name)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }
    };
}

name!(
    /// A publisher's id, unique in the deployment (`UniquePublisherId`).
    PublisherId,
    "p1"
);
name!(
    /// A consumer's id, unique in the deployment (`UniqueConsumerId`).
    ConsumerId,
    "c1"
);
name!(
    /// A pocket's key, unique in the queue (`OnePocketPerKey`).
    PocketKey,
    "tenant-a"
);
name!(
    /// A record's key, unique in its pocket (`OneRecordPerKeyPerPocket`).
    RecordKey,
    "k1"
);

/// The publisher-supplied order between the versions of one key; higher wins.
///
/// ```
/// use libpachinko::Rank;
/// assert!(Rank::new(2) > Rank::new(1));
/// assert_eq!(Rank::new(-3).get(), -3);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Rank(i64);

impl Rank {
    /// The rank with the given value.
    ///
    /// ```
    /// use libpachinko::Rank;
    /// assert_eq!(Rank::new(5), Rank::new(5));
    /// assert!(Rank::new(-1) < Rank::new(0));
    /// ```
    #[must_use]
    pub const fn new(rank: i64) -> Self {
        Self(rank)
    }

    /// The rank's value.
    ///
    /// ```
    /// assert_eq!(libpachinko::Rank::new(7).get(), 7);
    /// ```
    #[must_use]
    pub const fn get(self) -> i64 {
        self.0
    }
}

impl fmt::Display for Rank {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// The opaque bytes a record carries; pachinko never reads them.
///
/// ```
/// use libpachinko::Payload;
/// let payload = Payload::from(b"hello".as_slice());
/// assert_eq!(payload.as_bytes(), b"hello");
/// assert_eq!(payload.into_bytes(), b"hello".to_vec());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Payload(Vec<u8>);

impl Payload {
    /// The payload carrying `bytes`.
    ///
    /// ```
    /// use libpachinko::Payload;
    /// assert_eq!(Payload::new(vec![1, 2]).as_bytes(), &[1, 2]);
    /// ```
    #[must_use]
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }

    /// The bytes.
    ///
    /// ```
    /// use libpachinko::Payload;
    /// assert_eq!(Payload::from(b"ab".as_slice()).as_bytes(), b"ab");
    /// assert!(Payload::default().as_bytes().is_empty());
    /// ```
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The bytes, by value.
    ///
    /// ```
    /// use libpachinko::Payload;
    /// assert_eq!(Payload::new(vec![9]).into_bytes(), vec![9]);
    /// ```
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.0
    }
}

impl From<Vec<u8>> for Payload {
    fn from(bytes: Vec<u8>) -> Self {
        Self(bytes)
    }
}

impl From<&[u8]> for Payload {
    fn from(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }
}

/// Where a pocket stands in the pocket order and a record in its pocket: the
/// queue's counter when it entered or re-entered, smallest first.
///
/// ```
/// use libpachinko::Position;
/// assert!(Position::new(0) < Position::new(1));
/// assert_eq!(Position::new(4).get(), 4);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct Position(u64);

impl Position {
    /// The position with the given value.
    ///
    /// ```
    /// use libpachinko::Position;
    /// assert_eq!(Position::new(3), Position::new(3));
    /// assert!(Position::new(3) < Position::new(4));
    /// ```
    #[must_use]
    pub const fn new(position: u64) -> Self {
        Self(position)
    }

    /// The position's value.
    ///
    /// ```
    /// assert_eq!(libpachinko::Position::new(8).get(), 8);
    /// ```
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{ConsumerId, Payload, PocketKey, Position, PublisherId, Rank, RecordKey};

    #[test]
    fn names_convert_from_text_and_display_as_it() {
        assert_eq!(PublisherId::from(String::from("p")).as_str(), "p");
        assert_eq!(ConsumerId::new("c").to_string(), "c");
        assert!(PocketKey::from("a") < PocketKey::from("b"));
        assert_eq!(RecordKey::new(String::from("k")), RecordKey::from("k"));
    }

    #[test]
    fn ranks_and_positions_order_by_value() {
        assert!(Rank::new(-1) < Rank::new(0));
        assert_eq!(Rank::new(5).to_string(), "5");
        assert!(Position::new(3) > Position::default());
    }

    #[test]
    fn payloads_hold_bytes() {
        assert_eq!(Payload::new(vec![1, 2]).as_bytes(), [1, 2]);
        assert_eq!(Payload::from(vec![3]).into_bytes(), [3]);
        assert_eq!(Payload::default().as_bytes(), b"");
    }
}
