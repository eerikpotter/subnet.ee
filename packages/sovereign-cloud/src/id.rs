//! Canonical catalog identity. Text on the wire, sixteen bytes in stable indexes.
use candid::{
    CandidType,
    types::{Serializer, Type},
};
use serde::{Deserialize, Deserializer, Serialize, Serializer as SerdeSerializer};
use std::{fmt, str::FromStr};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId([u8; 16]);

impl ObjectId {
    /// Range sentinels, never allocated as object identities.
    pub const MIN: Self = Self([0; 16]);
    pub const MAX: Self = Self([255; 16]);

    /// The caller supplies cryptographically random bytes, not a timestamp/counter.
    pub fn from_random_bytes(bytes: [u8; 16]) -> Self {
        Self(*uuid::Builder::from_random_bytes(bytes).into_uuid().as_bytes())
    }
    pub const fn as_bytes(&self) -> &[u8; 16] {
        &self.0
    }
}
impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        uuid::Uuid::from_bytes(self.0).hyphenated().fmt(f)
    }
}
impl FromStr for ObjectId {
    type Err = &'static str;
    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let id = uuid::Uuid::parse_str(value).map_err(|_| "Invalid object UUID")?;
        if value.len() != 36 || id.get_version_num() != 4 || id.get_variant() != uuid::Variant::RFC4122 {
            return Err("Expected a version 4 object UUID");
        }
        Ok(Self(*id.as_bytes()))
    }
}
impl Serialize for ObjectId {
    fn serialize<S: SerdeSerializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}
impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?.parse().map_err(serde::de::Error::custom)
    }
}
impl CandidType for ObjectId {
    fn _ty() -> Type {
        String::_ty()
    }
    fn idl_serialize<S: Serializer>(&self, serializer: S) -> Result<(), S::Error> {
        serializer.serialize_text(&self.to_string())
    }
}

#[cfg(feature = "storage")]
impl ic_stable_structures::Storable for ObjectId {
    const BOUND: ic_stable_structures::storable::Bound = ic_stable_structures::storable::Bound::Bounded { max_size: 16, is_fixed_size: true };
    fn to_bytes(&self) -> std::borrow::Cow<'_, [u8]> {
        std::borrow::Cow::Borrowed(&self.0)
    }
    fn into_bytes(self) -> Vec<u8> {
        self.0.to_vec()
    }
    fn from_bytes(bytes: std::borrow::Cow<'_, [u8]>) -> Self {
        // Only stable-map keys written by this implementation reach this method.
        let mut raw = [0; 16];
        raw.copy_from_slice(&bytes);
        Self(raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn identity_roundtrips_as_text_without_integer_coercion() {
        let id = ObjectId::from_random_bytes([123; 16]);
        let wire = candid::encode_one(id).unwrap();
        assert_eq!(candid::decode_one::<String>(&wire).unwrap(), id.to_string());
        assert_eq!(candid::decode_one::<ObjectId>(&wire).unwrap(), id);
        assert_eq!(serde_json::from_str::<ObjectId>(&serde_json::to_string(&id).unwrap()).unwrap(), id);
        for invalid in ["8", "", "00000000-0000-0000-0000-000000000000", "7b7b7b7b7b7b4b7bbb7b7b7b7b7b7b7b"] {
            assert!(invalid.parse::<ObjectId>().is_err());
        }
    }
}
