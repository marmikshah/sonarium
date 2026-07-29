//! Stable identifier newtypes for the compiled program (ADR 0003).
//!
//! Each wraps a `u64` and exists so a track, pattern, placement, parameter,
//! or bus can be referenced by a cheap, copyable handle instead of a string.
//! The IDs are assigned deterministically at compile time in declaration
//! order, so recompiling an unchanged song reproduces IDENTICAL ids — that is
//! their entire purpose. The assignment itself happens in the compiler, not
//! here; this module is just the typed wrappers.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Identifies one instrument track of a compiled song.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct TrackId(pub u64);

/// Identifies one reusable pattern of a compiled song.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct PatternId(pub u64);

/// Identifies one placement (a pattern arranged onto a track) of a compiled
/// song.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct PlacementId(pub u64);

/// Identifies one automatable parameter of a compiled song.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct ParamId(pub u64);

/// Identifies one mix bus of a compiled song.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct BusId(pub u64);

impl TrackId {
    /// The raw numeric id.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl PatternId {
    /// The raw numeric id.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl PlacementId {
    /// The raw numeric id.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl ParamId {
    /// The raw numeric id.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl BusId {
    /// The raw numeric id.
    pub fn get(self) -> u64 {
        self.0
    }
}

impl From<u64> for TrackId {
    fn from(n: u64) -> Self {
        TrackId(n)
    }
}

impl From<u64> for PatternId {
    fn from(n: u64) -> Self {
        PatternId(n)
    }
}

impl From<u64> for PlacementId {
    fn from(n: u64) -> Self {
        PlacementId(n)
    }
}

impl From<u64> for ParamId {
    fn from(n: u64) -> Self {
        ParamId(n)
    }
}

impl From<u64> for BusId {
    fn from(n: u64) -> Self {
        BusId(n)
    }
}

impl std::fmt::Display for TrackId {
    /// The bare number (`3`), so an id reads the same in logs as in JSON.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for PatternId {
    /// The bare number (`3`), so an id reads the same in logs as in JSON.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for PlacementId {
    /// The bare number (`3`), so an id reads the same in logs as in JSON.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for ParamId {
    /// The bare number (`3`), so an id reads the same in logs as in JSON.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::fmt::Display for BusId {
    /// The bare number (`3`), so an id reads the same in logs as in JSON.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serde_round_trip_is_the_bare_number() {
        assert_eq!(serde_json::to_string(&TrackId::from(3)).unwrap(), "3");
        assert_eq!(serde_json::to_string(&BusId::from(9)).unwrap(), "9");
        let id: TrackId = serde_json::from_str("3").unwrap();
        assert_eq!(id, TrackId(3));
        assert_eq!(id.get(), 3);
    }

    #[test]
    fn display_is_the_bare_number() {
        assert_eq!(TrackId(3).to_string(), "3");
        assert_eq!(PlacementId(17).to_string(), "17");
    }
}
