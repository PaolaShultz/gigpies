//! C-ROLE:1 intended assignment registry; injected native leases are separate.
pub mod native;
use crate::show::{Counter, Result, decode, id, persist};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::Path};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    AudioDesk,
    LightingDesk,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub role: Role,
    pub display_connector: String,
    pub display_edid: String,
    pub controller: String,
    pub profile: String,
    /// Required when serial/EDID identity is ambiguous; chosen by operator.
    pub operator_label: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub format: String,
    pub version: u32,
    pub generation: Counter,
    pub bindings: Vec<Binding>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub connector: String,
    pub edid: String,
}
/// Ambiguous EDIDs remain unbound; explicit operator choices use claim().
pub fn unambiguous_displays(displays: &[Display]) -> Vec<Display> {
    displays
        .iter()
        .filter(|d| {
            displays.iter().filter(|other| other.edid == d.edid).count() == 1
                && displays
                    .iter()
                    .filter(|other| other.connector == d.connector)
                    .count()
                    == 1
        })
        .cloned()
        .collect()
}
impl Registry {
    pub fn decode(bytes: &[u8]) -> Result<Self> {
        let r: Self = decode(bytes)?;
        r.validate()?;
        Ok(r)
    }
    pub fn validate(&self) -> Result<()> {
        if self.format != "gigpies-roles" || self.version != 1 {
            return Err("version".into());
        }
        if self.bindings.len() > 2 {
            return Err("capacity".into());
        }
        let mut roles = BTreeSet::new();
        let mut controllers = BTreeSet::new();
        let mut connectors = BTreeSet::new();
        for b in &self.bindings {
            if !roles.insert(b.role as u8)
                || !controllers.insert(&b.controller)
                || !connectors.insert(&b.display_connector)
                || [
                    &b.display_connector,
                    &b.display_edid,
                    &b.controller,
                    &b.profile,
                ]
                .iter()
                .any(|s| !id(s))
                || b.operator_label.as_ref().is_some_and(|s| {
                    s.is_empty() || s.len() > 128 || s.chars().any(char::is_control)
                })
            {
                return Err("identity".into());
            }
        }
        for b in &self.bindings {
            if self
                .bindings
                .iter()
                .any(|other| other.role != b.role && other.display_edid == b.display_edid)
                && (b.operator_label.is_none()
                    || self.bindings.iter().any(|other| {
                        other.role != b.role
                            && other.display_edid == b.display_edid
                            && other.operator_label == b.operator_label
                    }))
            {
                return Err("ambiguous_identity".into());
            }
        }
        Ok(())
    }
    /// Verified identity/profile data supplied by caller; no device is opened.
    /// Any refusal preserves all registry bytes, including generation.
    pub fn claim(&mut self, expected: Counter, binding: Binding) -> Result<Counter> {
        self.validate()?;
        if expected != self.generation {
            return Err("stale_generation".into());
        }
        if self.bindings.iter().any(|b| {
            b.role == binding.role
                || b.controller == binding.controller
                || b.display_connector == binding.display_connector
        }) {
            return Err("conflict".into());
        }
        if self
            .bindings
            .iter()
            .any(|b| b.display_edid == binding.display_edid)
            && (binding.operator_label.is_none()
                || self
                    .bindings
                    .iter()
                    .any(|b| b.display_edid == binding.display_edid && b.operator_label.is_none()))
        {
            return Err("ambiguous_identity".into());
        }
        let mut next = self.clone();
        next.generation = Counter(
            self.generation
                .0
                .checked_add(1)
                .ok_or("generation exhausted")?,
        );
        next.bindings.push(binding);
        next.validate()?;
        *self = next;
        Ok(self.generation)
    }
    pub fn release(&mut self, expected: Counter, role: Role, controller: &str) -> Result<Counter> {
        self.validate()?;
        if expected != self.generation {
            return Err("stale_generation".into());
        }
        let index = self
            .bindings
            .iter()
            .position(|b| b.role == role && b.controller == controller)
            .ok_or("identity")?;
        let generation = Counter(
            self.generation
                .0
                .checked_add(1)
                .ok_or("generation exhausted")?,
        );
        self.bindings.remove(index);
        self.generation = generation;
        Ok(generation)
    }
    pub fn save(&self, directory: &Path, name: &str) -> Result<()> {
        self.validate()?;
        persist(directory, name, self)
    }
}
