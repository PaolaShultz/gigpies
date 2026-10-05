//! C-SHOW:1 portable compatibility metadata. Loading never recalls engine state.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

pub type Result<T> = std::result::Result<T, String>;

/// Decimal-string u64, preserving identity in JSON consumers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Counter(pub u64);
impl Serialize for Counter {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.0.to_string())
    }
}
impl<'de> Deserialize<'de> for Counter {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        if s.is_empty()
            || !s.bytes().all(|b| b.is_ascii_digit())
            || (s.len() > 1 && s.starts_with('0'))
        {
            return Err(serde::de::Error::custom(
                "canonical decimal string required",
            ));
        }
        s.parse().map(Self).map_err(serde::de::Error::custom)
    }
}

pub(crate) fn id(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 64
        && s.bytes().enumerate().all(|(i, b)| {
            b.is_ascii_lowercase() || b.is_ascii_digit() || (i > 0 && b"._-".contains(&b))
        })
}
fn known_contract(s: &str) -> bool {
    matches!(
        s,
        "C-AUDIO" | "C-LIGHT" | "C-REC" | "C-FX" | "C-PA" | "C-ANALYSIS" | "C-SHOW" | "C-ROLE"
    )
}
fn uuid(s: &str) -> bool {
    s.len() == 36
        && s.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub id: String,
    pub contract: String,
    pub version: u32,
    pub required: bool,
    /// SHA-256 of separately owned state; never an endpoint or hardware path.
    pub content_identity: String,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Manifest {
    pub format: String,
    pub version: u32,
    pub show_id: String,
    pub manifest_revision: Counter,
    pub modules: Vec<Module>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Availability {
    Compatible,
    Unavailable,
    Version,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Attachment {
    pub read_only: bool,
    pub modules: Vec<(String, Availability)>,
}
impl Manifest {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let m: Self = decode(bytes)?;
        m.validate()?;
        Ok(m)
    }
    pub fn validate(&self) -> Result<()> {
        if self.format != "gigpies-show" || self.version != 1 {
            return Err("version".into());
        }
        if !uuid(&self.show_id) || self.manifest_revision.0 == 0 || self.modules.len() > 16 {
            return Err("identity or capacity".into());
        }
        let mut seen = BTreeSet::new();
        for m in &self.modules {
            let known = known_contract(&m.contract);
            if m.required && (!known || m.version != 1) {
                return Err("version".into());
            }
            if !id(&m.id)
                || !seen.insert(&m.id)
                || m.contract.is_empty()
                || m.contract.len() > 64
                || !m
                    .contract
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b == b'-')
                || m.content_identity.len() != 64
                || !m
                    .content_identity
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err("module identity".into());
            }
        }
        Ok(())
    }
    /// Provider capabilities are data only; mismatch prevents writable attachment.
    pub fn attach(&self, providers: &[(&str, &str, u32)]) -> Result<Attachment> {
        self.validate()?;
        let mut modules = Vec::new();
        let mut read_only = false;
        for m in &self.modules {
            let matching: Vec<_> = providers.iter().filter(|p| p.0 == m.id).collect();
            if matching.len() > 1 {
                return Err("duplicate provider".into());
            }
            let availability = match matching.first() {
                _ if !known_contract(&m.contract) => Availability::Unavailable,
                None if m.required => return Err("missing required module".into()),
                None => Availability::Unavailable,
                Some(p) if p.1 != m.contract || p.2 != m.version => Availability::Version,
                Some(_) => Availability::Compatible,
            };
            read_only |= m.required && availability != Availability::Compatible;
            modules.push((m.id.clone(), availability));
        }
        Ok(Attachment { read_only, modules })
    }
    pub fn save(&self, directory: &Path, name: &str) -> Result<()> {
        self.validate()?;
        persist(directory, name, self)
    }
}

/// Reject duplicate keys (including nested ones) before typed deserialization.
pub(crate) fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T> {
    decode_bounded(bytes, 65536)
}
pub(crate) fn decode_bounded<T: serde::de::DeserializeOwned>(
    bytes: &[u8],
    max_bytes: usize,
) -> Result<T> {
    if bytes.len() > max_bytes {
        return Err("capacity".into());
    }
    struct Checked;
    impl<'de> serde::de::Visitor<'de> for Checked {
        type Value = ();
        fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            f.write_str("bounded JSON")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut a: A,
        ) -> std::result::Result<(), A::Error> {
            let mut keys = BTreeSet::new();
            while let Some(k) = a.next_key::<String>()? {
                if !keys.insert(k) {
                    return Err(serde::de::Error::custom("duplicate key"));
                }
                a.next_value::<CheckedValue>()?;
            }
            Ok(())
        }
        fn visit_seq<A: serde::de::SeqAccess<'de>>(
            self,
            mut a: A,
        ) -> std::result::Result<(), A::Error> {
            while a.next_element::<CheckedValue>()?.is_some() {}
            Ok(())
        }
        fn visit_bool<E: serde::de::Error>(self, _: bool) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_i64<E: serde::de::Error>(self, _: i64) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_u64<E: serde::de::Error>(self, _: u64) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_f64<E: serde::de::Error>(self, _: f64) -> std::result::Result<(), E> {
            Err(E::custom("integer quantities only"))
        }
        fn visit_str<E: serde::de::Error>(self, _: &str) -> std::result::Result<(), E> {
            Ok(())
        }
        fn visit_unit<E: serde::de::Error>(self) -> std::result::Result<(), E> {
            Ok(())
        }
    }
    struct CheckedValue;
    impl<'de> Deserialize<'de> for CheckedValue {
        fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
            d.deserialize_any(Checked)?;
            Ok(Self)
        }
    }
    // Scan nesting outside strings, before recursive parsing.
    let mut depth = 0usize;
    let mut quoted = false;
    let mut escape = false;
    for &b in bytes {
        if quoted {
            if escape {
                escape = false;
            } else if b == b'\\' {
                escape = true;
            } else if b == b'"' {
                quoted = false;
            }
        } else {
            match b {
                b'"' => quoted = true,
                b'{' | b'[' => {
                    depth += 1;
                    if depth > 12 {
                        return Err("depth".into());
                    }
                }
                b'}' | b']' => depth = depth.saturating_sub(1),
                _ => {}
            }
        }
    }
    let mut d = serde_json::Deserializer::from_slice(bytes);
    CheckedValue::deserialize(&mut d).map_err(|e| e.to_string())?;
    d.end().map_err(|e| e.to_string())?;
    serde_json::from_slice(bytes).map_err(|e| e.to_string())
}

/// Trusted caller supplies an owned private directory and simple filename.
/// Failure after rename reports uncertain durability; it never claims rollback.
pub(crate) fn persist<T: Serialize>(directory: &Path, name: &str, value: &T) -> Result<()> {
    persist_bounded(directory, name, value, 65536)
}
pub(crate) fn persist_bounded<T: Serialize>(
    directory: &Path,
    name: &str,
    value: &T,
    max_bytes: usize,
) -> Result<()> {
    use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
    if !id(name) {
        return Err("filename".into());
    }
    let meta = fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
    if meta.uid() != fs::metadata("/proc/self").map_err(|e| e.to_string())?.uid()
        || !meta.is_dir()
        || meta.permissions().mode() & 0o777 != 0o700
    {
        return Err("owned 0700 directory required".into());
    }
    let bytes = serde_json::to_vec_pretty(value).map_err(|e| e.to_string())?;
    if bytes.len() > max_bytes {
        return Err("capacity".into());
    }
    let mut temp = None;
    for n in 0..64 {
        let path = directory.join(format!(".{name}.{}.{n}.tmp", std::process::id()));
        match OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
        {
            Ok(file) => {
                temp = Some((path, file));
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    let (path, mut file) = temp.ok_or("temporary capacity")?;
    let result = (|| {
        file.write_all(&bytes)?;
        file.flush()?;
        file.sync_all()?;
        fs::rename(&path, directory.join(name))?;
        fs::File::open(directory)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&path);
    }
    result.map_err(|e| format!("checkpoint error (replacement may have completed): {e}"))
}
