//! The specification's names and values: who publishes and consumes, which pocket
//! and key a record has, its rank and payload, a held record as a batch carries it,
//! and the id of a lease, the fencing token every ack, nack, renew and release
//! carries back, with the one text form the wire carries it in.

use std::fmt;
use std::str::FromStr;

/// Declares a newtype over `String` that names something in the specification.
macro_rules! name {
    ($(#[$doc:meta])* $name:ident, $example:literal) => {
        $(#[$doc])*
        ///
        /// ```
        #[doc = concat!("use libpachinko::", stringify!($name), ";")]
        #[doc = concat!("let name = ", stringify!($name), "::new(\"", $example, "\");")]
        #[doc = concat!("assert_eq!(name.as_str(), \"", $example, "\");")]
        #[doc = concat!("assert_eq!(name.to_string(), \"", $example, "\");")]
        #[doc = concat!("assert_eq!(name, ", stringify!($name), "::from(\"", $example, "\"));")]
        /// ```
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[non_exhaustive]
        pub struct $name(String);

        impl $name {
            /// The name, from any string.
            ///
            /// ```
            #[doc = concat!("use libpachinko::", stringify!($name), ";")]
            #[doc = concat!("let name = ", stringify!($name), "::new(String::from(\"", $example, "\"));")]
            #[doc = concat!("assert_eq!(name.as_str(), \"", $example, "\");")]
            /// ```
            #[must_use]
            pub fn new(name: impl Into<String>) -> Self {
                Self(name.into())
            }

            /// The name as text.
            ///
            /// ```
            #[doc = concat!("use libpachinko::", stringify!($name), ";")]
            #[doc = concat!("assert_eq!(", stringify!($name), "::new(\"", $example, "\").as_str(), \"", $example, "\");")]
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

/// The publisher-supplied order between the versions of one key; higher wins. It
/// orders like the `i64` it holds, negatives and zero included.
///
/// ```
/// use libpachinko::Rank;
/// assert!(Rank::new(2) > Rank::new(1));
/// assert!(Rank::new(-1) < Rank::new(0));
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
    /// assert_eq!(Rank::new(i64::MAX).get(), i64::MAX);
    /// ```
    #[must_use]
    pub const fn new(rank: i64) -> Self {
        Self(rank)
    }

    /// The rank's value.
    ///
    /// ```
    /// use libpachinko::Rank;
    /// assert_eq!(Rank::new(7).get(), 7);
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
/// assert_eq!(payload.as_ref(), b"hello");
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
    /// assert_eq!(Payload::new(vec![1, 2]).as_ref(), &[1, 2]);
    /// ```
    #[must_use]
    pub const fn new(bytes: Vec<u8>) -> Self {
        Self(bytes)
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

impl AsRef<[u8]> for Payload {
    fn as_ref(&self) -> &[u8] {
        &self.0
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

/// A held record as a batch carries it and the `Consuming` surface exposes it: the
/// key, the held rank and the held payload.
///
/// ```
/// use libpachinko::{HeldRecord, Payload, Rank, RecordKey};
///
/// let held = HeldRecord::new(RecordKey::new("k"), Rank::new(2), Payload::default());
/// assert_eq!(held.key.as_str(), "k");
/// assert_eq!(held.rank, Rank::new(2));
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct HeldRecord {
    /// The record key.
    pub key: RecordKey,
    /// The rank of the version handed out.
    pub rank: Rank,
    /// The payload of the version handed out.
    pub payload: Payload,
}

impl HeldRecord {
    /// A held record with these parts.
    ///
    /// ```
    /// use libpachinko::{HeldRecord, Payload, Rank, RecordKey};
    ///
    /// assert_eq!(HeldRecord::new(RecordKey::new("k"), Rank::new(7), Payload::default()).rank.get(), 7);
    /// ```
    #[must_use]
    pub const fn new(key: RecordKey, rank: Rank, payload: Payload) -> Self {
        Self { key, rank, payload }
    }
}

/// The identity of one lease, opaque to its holder: a `u128` whose high 64 bits are
/// the source's epoch and whose low 64 bits are its serial.
///
/// It is written as exactly 32 lower-case hexadecimal digits, which is how the wire
/// carries it, and parsed back from exactly that form: a shorter or longer text, an
/// upper-case digit or a sign is refused.
///
/// ```
/// use libpachinko::LeaseId;
///
/// let id = LeaseId::from_u128(0xab);
/// assert_eq!(id.to_string(), "000000000000000000000000000000ab");
/// assert_eq!(id.to_string().parse::<LeaseId>(), Ok(id));
/// assert!("ab".parse::<LeaseId>().is_err());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub struct LeaseId(u128);

impl LeaseId {
    /// The lease id with the given 128 bits.
    ///
    /// ```
    /// use libpachinko::LeaseId;
    /// assert_eq!(LeaseId::from_u128(7).as_u128(), 7);
    /// ```
    #[must_use]
    pub const fn from_u128(bits: u128) -> Self {
        Self(bits)
    }

    /// The id's 128 bits.
    ///
    /// ```
    /// use libpachinko::LeaseId;
    /// assert_eq!(LeaseId::from_u128(9).as_u128(), 9);
    /// ```
    #[must_use]
    pub const fn as_u128(self) -> u128 {
        self.0
    }
}

impl fmt::Display for LeaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:032x}", self.0)
    }
}

/// Why a text is not a lease id.
///
/// ```
/// use libpachinko::{LeaseId, ParseLeaseIdError};
/// assert_eq!("xyz".parse::<LeaseId>(), Err(ParseLeaseIdError::NotThirtyTwoHexDigits));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ParseLeaseIdError {
    /// The text is not exactly 32 lower-case hexadecimal digits.
    #[error("a lease id is exactly 32 lower-case hexadecimal digits")]
    NotThirtyTwoHexDigits,
}

impl FromStr for LeaseId {
    type Err = ParseLeaseIdError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let well_formed =
            text.len() == 32 && text.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'));
        well_formed
            .then(|| u128::from_str_radix(text, 16).ok())
            .flatten()
            .map(Self)
            .ok_or(ParseLeaseIdError::NotThirtyTwoHexDigits)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ConsumerId, HeldRecord, LeaseId, ParseLeaseIdError, Payload, PocketKey, PublisherId, Rank,
        RecordKey,
    };

    #[test]
    fn names_convert_from_text_and_display_as_it() {
        assert_eq!(PublisherId::from(String::from("p")).as_str(), "p");
        assert_eq!(ConsumerId::new("c").to_string(), "c");
        assert!(PocketKey::from("a") < PocketKey::from("b"));
        assert_eq!(RecordKey::new(String::from("k")), RecordKey::from("k"));
    }

    #[test]
    fn ranks_order_by_value() {
        assert!(Rank::new(i64::MIN) < Rank::new(i64::MAX));
        assert_eq!(Rank::new(5).to_string(), "5");
    }

    #[test]
    fn payloads_hold_bytes() {
        assert_eq!(Payload::from(vec![3]).into_bytes(), [3]);
        assert_eq!(Payload::default().as_ref(), b"");
    }

    #[test]
    fn a_held_record_keeps_its_parts() {
        let held = HeldRecord::new(RecordKey::new("k"), Rank::new(2), Payload::from(vec![1]));
        assert_eq!(
            (held.key.as_str(), held.rank, held.payload.as_ref()),
            ("k", Rank::new(2), [1].as_slice())
        );
    }

    #[test]
    fn display_and_parse_round_trip_the_edges() {
        for bits in [
            0,
            1,
            u128::from(u64::MAX),
            u128::from(u64::MAX) + 1,
            u128::MAX,
        ] {
            let id = LeaseId::from_u128(bits);
            assert_eq!(id.to_string().len(), 32);
            assert_eq!(id.to_string().parse(), Ok(id));
        }
        assert_eq!(LeaseId::from_u128(u128::MAX).to_string(), "f".repeat(32));
    }

    #[test]
    fn parsing_refuses_every_other_form() {
        let upper = "A".repeat(32);
        let plus = format!("+{}", "0".repeat(31));
        let short = "0".repeat(31);
        let long = "0".repeat(33);
        let not_hex = "g".repeat(32);
        for bad in ["", "1", &short, &long, &not_hex, &upper, &plus] {
            assert_eq!(
                bad.parse::<LeaseId>(),
                Err(ParseLeaseIdError::NotThirtyTwoHexDigits),
                "{bad:?}"
            );
        }
        assert_eq!(
            ParseLeaseIdError::NotThirtyTwoHexDigits.to_string(),
            "a lease id is exactly 32 lower-case hexadecimal digits"
        );
    }
}
