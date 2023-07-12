use chrono::{DateTime, Utc};
use color_eyre::eyre::ContextCompat;
use color_eyre::Result;
use iroha_crypto::{Hash, HashOf, PublicKey, Signature};
use parity_scale_codec::Encode;
use serde::{de, Serialize};
use std::{fmt, marker::PhantomData};

/// Serializes into RFC 3339 and ISO 8601 format. Can be constructed from `u64` and `u128`.
///
/// ```ignore
/// let timestamp = Timestamp::try_from(1653584876961u128).unwrap();
/// let json = serde_json::to_string(&timestamp).unwrap();
/// assert_eq!(json, "2022-05-26T17:07:56.961Z")
/// ```
/// *(ignoring this doctest due [rust-lang/rust#50784](https://github.com/rust-lang/rust/issues/50784)*
#[derive(Serialize)]
pub struct Timestamp(DateTime<Utc>);

/// Input - unix time in milliseconds
impl TryFrom<u128> for Timestamp {
    type Error = color_eyre::Report;

    fn try_from(unix_time: u128) -> Result<Self> {
        let secs: i64 = (unix_time / 1_000).try_into()?;
        let nano_secs: u32 = ((unix_time % 1_000) * 1_000_000).try_into()?;
        let naive_dt = chrono::NaiveDateTime::from_timestamp_opt(secs, nano_secs)
            .wrap_err("Failed to construct NaiveDateTime")?;
        let dt = DateTime::<Utc>::from_utc(naive_dt, Utc);

        Ok(Self(dt))
    }
}

/// Input - unix time in milliseconds
impl TryFrom<u64> for Timestamp {
    type Error = color_eyre::Report;

    fn try_from(unix_time: u64) -> Result<Self> {
        Self::try_from(u128::from(unix_time))
    }
}

/// Container for a SCALE-encodable value. Serializes into hex string.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub struct SerScaleHex<T>(pub T)
where
    T: Encode;

impl<T> Serialize for SerScaleHex<T>
where
    T: Encode,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let bytes = self.0.encode();
        let value = hex::encode(bytes);
        serializer.serialize_str(&value)
    }
}

impl<T> From<T> for SerScaleHex<T>
where
    T: Encode,
{
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> From<HashOf<T>> for SerScaleHex<Hash> {
    fn from(value: HashOf<T>) -> Self {
        let hash: Hash = value.into();
        hash.into()
    }
}
// implementation for payload
impl From<&[u8]> for SerScaleHex<Vec<u8>> {
    fn from(value: &[u8]) -> Self {
        SerScaleHex(value.to_vec())
    }
}


/// Wrap that serializes into string. It's purpose is to add semantics
/// to serializable structures about **what** a particular data
/// is a string of.
///
/// ```ignore
/// struct Account {
///   // What this string actually is?
///   id_opaque: String,
///   
///   // Here it is clear, what
///   id_clear: StringOf<AccountId>
/// }
/// ```
///
/// It's generic type exists only for semantic reasons - `StringOf<T>` doesn't
/// actually own the `T`.
pub struct StringOf<T> {
    value: String,
    _marker: PhantomData<T>,
}

impl<T> From<T> for StringOf<T>
where
    T: ToString,
{
    fn from(value: T) -> Self {
        Self::from(&value)
    }
}

impl<T> From<&T> for StringOf<T>
where
    T: ToString,
{
    fn from(value: &T) -> Self {
        Self {
            value: value.to_string(),
            _marker: PhantomData,
        }
    }
}

impl<T> Serialize for StringOf<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.value.as_ref())
    }
}

/// Deserializes from string to [`Hash`].
pub struct HashDeser(pub Hash);

impl<'de> de::Deserialize<'de> for HashDeser {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        const HASH_HEX_LENGTH: usize = Hash::LENGTH * 2;

        struct Visitor;

        impl<'de> de::Visitor<'de> for Visitor {
            type Value = HashDeser;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a {}-byte hex string", Hash::LENGTH)
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v.len() != HASH_HEX_LENGTH {
                    return Err(E::invalid_value(de::Unexpected::Str(v), &self));
                }

                let mut slice = [0u8; Hash::LENGTH];
                hex::decode_to_slice(v, &mut slice)
                    .map_err(|_from_hex_error| E::invalid_value(de::Unexpected::Str(v), &self))?;
                let hash = Hash::prehashed(slice);
                Ok(HashDeser(hash))
            }
        }

        deserializer.deserialize_string(Visitor)
    }
}

/// Same as [`Signature`], but serializes payload as hex
#[derive(Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct SignatureDTO {
    pub public_key: PublicKey,
    pub payload: SerScaleHex<Vec<u8>>,
}

impl From<Signature> for SignatureDTO {
    fn from(value: Signature) -> Self {
        Self {
            public_key: value.public_key().clone(),
            payload: value.payload().clone().into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SerScaleHex, Timestamp};

    // TODO move to doctest when possible
    #[test]
    fn timestamp_from_unix_time() {
        let unix_millis_input = 1_653_584_876_961_u128;
        let expected_iso = "2022-05-26T17:07:56.961Z";
        let expected_iso_json = serde_json::to_string(&expected_iso).unwrap();

        let actual = Timestamp::try_from(unix_millis_input).unwrap();
        let actual_json = serde_json::to_string(&actual).unwrap();

        assert_eq!(actual_json, expected_iso_json);
    }

    // TODO move to doctest when possible
    #[test]
    fn scale_serialized_into_hex() {
        let sample_num = 42;
        let sample_num_expected_json = "\"2a000000\"";

        let wrap = SerScaleHex(sample_num);
        let wrap_json = serde_json::to_string(&wrap).unwrap();

        assert_eq!(wrap_json, sample_num_expected_json);
    }
}
