use serde::de::Error as _;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt::{Display, Formatter};

const UTF8_BOM: &[u8; 3] = b"\xef\xbb\xbf";
const FNV1A64_OFFSET: u64 = 0xcbf29ce484222325;
const FNV1A64_PRIME: u64 = 0x100000001b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PatchBaseDigest(u64);

impl PatchBaseDigest {
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut value = FNV1A64_OFFSET;
        for byte in bytes {
            value ^= u64::from(*byte);
            value = value.wrapping_mul(FNV1A64_PRIME);
        }
        Self(value)
    }

    #[must_use]
    pub const fn as_u64(self) -> u64 {
        self.0
    }
}

impl Display for PatchBaseDigest {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "fnv1a64:{:016x}", self.0)
    }
}

impl Serialize for PatchBaseDigest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for PatchBaseDigest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = String::deserialize(deserializer)?;
        let hex = wire
            .strip_prefix("fnv1a64:")
            .filter(|hex| hex.len() == 16)
            .ok_or_else(|| {
                D::Error::custom("patch base digest must use fnv1a64 plus 16 lowercase hex digits")
            })?;
        if !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
        {
            return Err(D::Error::custom(
                "patch base digest contains invalid hex digits",
            ));
        }
        let value = u64::from_str_radix(hex, 16).map_err(D::Error::custom)?;
        Ok(Self(value))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextLineEnding {
    None,
    Lf,
    CrLf,
    Mixed,
}

impl TextLineEnding {
    #[must_use]
    pub const fn sequence(self) -> Option<&'static str> {
        match self {
            Self::Lf => Some("\n"),
            Self::CrLf => Some("\r\n"),
            Self::None | Self::Mixed => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextDocumentBase {
    pub(super) digest: PatchBaseDigest,
    pub(super) encoding: TextEncoding,
    pub(super) line_ending: TextLineEnding,
    pub(super) byte_len: usize,
    pub(super) ends_with_line_ending: bool,
}

impl TextDocumentBase {
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self {
            digest: PatchBaseDigest::from_bytes(bytes),
            encoding: detect_encoding(bytes),
            line_ending: detect_line_ending(bytes),
            byte_len: bytes.len(),
            ends_with_line_ending: bytes.ends_with(b"\n") || bytes.ends_with(b"\r"),
        }
    }

    #[must_use]
    pub const fn digest(&self) -> PatchBaseDigest {
        self.digest
    }

    #[must_use]
    pub const fn encoding(&self) -> TextEncoding {
        self.encoding
    }

    #[must_use]
    pub const fn line_ending(&self) -> TextLineEnding {
        self.line_ending
    }

    #[must_use]
    pub const fn byte_len(&self) -> usize {
        self.byte_len
    }

    #[must_use]
    pub const fn ends_with_line_ending(&self) -> bool {
        self.ends_with_line_ending
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TextEditSpan {
    pub byte_start: usize,
    pub byte_end: usize,
}

impl TextEditSpan {
    #[must_use]
    pub const fn new(byte_start: usize, byte_end: usize) -> Option<Self> {
        if byte_start <= byte_end {
            Some(Self {
                byte_start,
                byte_end,
            })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn insertion(byte_offset: usize) -> Self {
        Self {
            byte_start: byte_offset,
            byte_end: byte_offset,
        }
    }

    #[must_use]
    pub const fn replaced_bytes(self) -> usize {
        self.byte_end - self.byte_start
    }
}

impl<'de> Deserialize<'de> for TextEditSpan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct WireSpan {
            byte_start: usize,
            byte_end: usize,
        }

        let wire = WireSpan::deserialize(deserializer)?;
        Self::new(wire.byte_start, wire.byte_end)
            .ok_or_else(|| D::Error::custom("text edit span start must not exceed end"))
    }
}

fn detect_encoding(bytes: &[u8]) -> TextEncoding {
    if std::str::from_utf8(bytes).is_err() {
        TextEncoding::Unknown
    } else if bytes.starts_with(UTF8_BOM) {
        TextEncoding::Utf8Bom
    } else {
        TextEncoding::Utf8
    }
}

pub(super) fn detect_line_ending(bytes: &[u8]) -> TextLineEnding {
    let mut has_lf = false;
    let mut has_crlf = false;
    let mut has_lone_cr = false;
    let mut cursor = 0;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'\r' if bytes.get(cursor + 1) == Some(&b'\n') => {
                has_crlf = true;
                cursor += 2;
            }
            b'\r' => {
                has_lone_cr = true;
                cursor += 1;
            }
            b'\n' => {
                has_lf = true;
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }

    match (has_lf, has_crlf, has_lone_cr) {
        (false, false, false) => TextLineEnding::None,
        (true, false, false) => TextLineEnding::Lf,
        (false, true, false) => TextLineEnding::CrLf,
        _ => TextLineEnding::Mixed,
    }
}
