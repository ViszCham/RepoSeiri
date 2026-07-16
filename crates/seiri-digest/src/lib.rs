#![forbid(unsafe_code)]

use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use sha2::{Digest, Sha256};
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Digest32([u8; 32]);

impl Digest32 {
    #[must_use]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn bytes(self) -> [u8; 32] {
        self.0
    }
}

impl Display for Digest32 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("sha256:")?;
        for byte in self.0 {
            write!(formatter, "{byte:02x}")?;
        }
        Ok(())
    }
}

impl Serialize for Digest32 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Digest32 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        let hex = value
            .strip_prefix("sha256:")
            .filter(|hex| hex.len() == 64)
            .ok_or_else(|| {
                D::Error::custom("digest must use sha256 plus 64 lowercase hex digits")
            })?;
        if !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(D::Error::custom("digest contains invalid hex digits"));
        }
        let mut bytes = [0u8; 32];
        for (index, pair) in hex.as_bytes().chunks_exact(2).enumerate() {
            let pair = std::str::from_utf8(pair).map_err(D::Error::custom)?;
            bytes[index] = u8::from_str_radix(pair, 16).map_err(D::Error::custom)?;
        }
        Ok(Self(bytes))
    }
}

/// A length-delimited, fixed-endian SHA-256 input builder.
///
/// It deliberately has no generic serialization entry point. Callers must
/// choose stable field tags and enum discriminants explicitly.
pub struct StableHasher(Sha256);

impl StableHasher {
    #[must_use]
    pub fn new(domain: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(b"RepoSeiri\0stable-digest\0v1");
        write_len(&mut hasher, domain.len());
        hasher.update(domain);
        Self(hasher)
    }

    pub fn field(&mut self, tag: u8, bytes: &[u8]) -> &mut Self {
        self.0.update([tag]);
        write_len(&mut self.0, bytes.len());
        self.0.update(bytes);
        self
    }

    pub fn str(&mut self, tag: u8, value: &str) -> &mut Self {
        self.field(tag, value.as_bytes())
    }

    pub fn bool(&mut self, tag: u8, value: bool) -> &mut Self {
        self.field(tag, &[u8::from(value)])
    }

    pub fn u8(&mut self, tag: u8, value: u8) -> &mut Self {
        self.field(tag, &[value])
    }

    pub fn u32(&mut self, tag: u8, value: u32) -> &mut Self {
        self.field(tag, &value.to_be_bytes())
    }

    pub fn u64(&mut self, tag: u8, value: u64) -> &mut Self {
        self.field(tag, &value.to_be_bytes())
    }

    pub fn usize(&mut self, tag: u8, value: usize) -> &mut Self {
        self.u64(tag, u64::try_from(value).unwrap_or(u64::MAX))
    }

    pub fn digest(&mut self, tag: u8, value: Digest32) -> &mut Self {
        self.field(tag, &value.bytes())
    }

    #[must_use]
    pub fn finish(self) -> Digest32 {
        Digest32::new(self.0.finalize().into())
    }
}

fn write_len(hasher: &mut Sha256, len: usize) {
    hasher.update(u64::try_from(len).unwrap_or(u64::MAX).to_be_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_are_ordered_length_delimited_and_domain_separated() {
        let mut first = StableHasher::new(b"domain-a");
        first.str(1, "ab").str(2, "c");
        let mut second = StableHasher::new(b"domain-a");
        second.str(1, "a").str(2, "bc");
        let mut other_domain = StableHasher::new(b"domain-b");
        other_domain.str(1, "ab").str(2, "c");

        assert_ne!(first.finish(), second.finish());
        let mut repeated = StableHasher::new(b"domain-a");
        repeated.str(1, "ab").str(2, "c");
        assert_ne!(repeated.finish(), other_domain.finish());
    }

    #[test]
    fn digest_wire_is_canonical() {
        let digest = Digest32::new([0xab; 32]);
        let wire = serde_json::to_string(&digest).expect("serialize digest");
        assert_eq!(wire, format!("\"{}\"", digest));
        assert_eq!(
            serde_json::from_str::<Digest32>(&wire).expect("deserialize digest"),
            digest
        );
        assert!(serde_json::from_str::<Digest32>("\"sha256:AB\"").is_err());
    }
}
