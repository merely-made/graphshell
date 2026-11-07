// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The vector sheet: what a guest block returns when its view is a drawing.
//!
//! A `sceno` scene names regions and carries no rendered content. A guest whose view is
//! engraved (notation, a diagram) returns a sheet beside the scene: ordered fills and
//! strokes over compound paths, paint named by theme role (never by colour), and text runs.
//! The host paints it with its own theme and fonts; the guest never draws and never
//! chooses a colour. The payload is a version byte then CBOR, at most [`MAX_PAYLOAD_BYTES`],
//! like the projection session's payloads.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Version byte of the first sheet.
pub const SHEET_VERSION: u8 = 1;
/// The cap every payload obeys (1 MiB).
pub const MAX_PAYLOAD_BYTES: usize = 1024 * 1024;
/// Most drawing operations in one sheet.
pub const MAX_OPS: usize = 50_000;
/// Most path segments in one sheet, across all operations.
pub const MAX_SEGMENTS: usize = 400_000;
/// Longest text run, in bytes.
pub const MAX_TEXT_BYTES: usize = 256;

/// How a fill decides what is inside a compound path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FillRule {
    NonZero,
    EvenOdd,
}

/// A theme role. The host maps it to its own colour; a guest cannot name a colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Primary ink: staff lines, note heads, digits.
    Ink,
    /// Emphasis: the selected event.
    Accent,
    /// De-emphasised ink.
    Muted,
}

/// Role plus opacity in `0..=1`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Paint {
    pub role: Role,
    pub alpha: f32,
}

impl Paint {
    pub const fn new(role: Role) -> Self {
        Self { role, alpha: 1.0 }
    }
    pub const fn with_alpha(role: Role, alpha: f32) -> Self {
        Self { role, alpha }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cap {
    Butt,
    Round,
    Square,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Join {
    Miter,
    Round,
    Bevel,
}

/// One path segment. A path is a sequence of subpaths, each starting with `Move`; `Close`
/// ends a subpath. Compound paths (counters, holes) are several subpaths in one path.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    /// Quadratic: control, then end point.
    Quad(f32, f32, f32, f32),
    /// Cubic: two controls, then end point.
    Cubic(f32, f32, f32, f32, f32, f32),
    Close,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Path {
    pub segs: Vec<Seg>,
}

impl Path {
    pub fn rect(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            segs: vec![
                Seg::Move(x, y),
                Seg::Line(x + w, y),
                Seg::Line(x + w, y + h),
                Seg::Line(x, y + h),
                Seg::Close,
            ],
        }
    }
}

/// Where a text run's `x` sits relative to the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Anchor {
    Start,
    Middle,
    End,
}

/// The host font family a text run asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Face {
    Reading,
    Mono,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Fill {
        path: Path,
        rule: FillRule,
        paint: Paint,
    },
    Stroke {
        path: Path,
        width: f32,
        cap: Cap,
        join: Join,
        paint: Paint,
    },
    /// Top-left of the text box at `(x, y)` for `Anchor::Start`; the host shapes the text.
    Text {
        x: f32,
        y: f32,
        size: f32,
        text: String,
        anchor: Anchor,
        face: Face,
        paint: Paint,
    },
}

/// An ordered drawing in a `width` by `height` box of device-independent pixels.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VectorSheet {
    pub width: f32,
    pub height: f32,
    pub ops: Vec<Op>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum SheetError {
    TooLarge(usize),
    Version(u8),
    Empty,
    Decode(String),
    Invalid(&'static str),
}

impl fmt::Display for SheetError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(n) => write!(f, "sheet payload of {n} bytes exceeds the 1 MiB cap"),
            Self::Version(v) => write!(f, "unsupported vector sheet version {v}"),
            Self::Empty => write!(f, "empty vector sheet payload"),
            Self::Decode(e) => write!(f, "vector sheet did not decode: {e}"),
            Self::Invalid(why) => write!(f, "invalid vector sheet: {why}"),
        }
    }
}
impl std::error::Error for SheetError {}

fn finite(v: f32) -> bool {
    v.is_finite() && v.abs() <= 1.0e6
}

fn seg_finite(seg: &Seg) -> bool {
    match *seg {
        Seg::Move(a, b) | Seg::Line(a, b) => finite(a) && finite(b),
        Seg::Quad(a, b, c, d) => [a, b, c, d].into_iter().all(finite),
        Seg::Cubic(a, b, c, d, e, g) => [a, b, c, d, e, g].into_iter().all(finite),
        Seg::Close => true,
    }
}

fn paint_valid(p: &Paint) -> bool {
    p.alpha.is_finite() && (0.0..=1.0).contains(&p.alpha)
}

impl VectorSheet {
    /// Check bounds before painting; a host refuses a sheet that fails.
    pub fn validate(&self) -> Result<(), SheetError> {
        if !(finite(self.width) && finite(self.height) && self.width > 0.0 && self.height > 0.0) {
            return Err(SheetError::Invalid("size must be positive and finite"));
        }
        if self.ops.len() > MAX_OPS {
            return Err(SheetError::Invalid("too many operations"));
        }
        let mut segments = 0usize;
        for op in &self.ops {
            match op {
                Op::Fill { path, paint, .. } | Op::Stroke { path, paint, .. } => {
                    segments += path.segs.len();
                    if !path.segs.iter().all(seg_finite) {
                        return Err(SheetError::Invalid("non-finite path coordinate"));
                    }
                    if !paint_valid(paint) {
                        return Err(SheetError::Invalid("alpha outside 0..=1"));
                    }
                    if let Op::Stroke { width, .. } = op {
                        if !(finite(*width) && *width >= 0.0) {
                            return Err(SheetError::Invalid("stroke width"));
                        }
                    }
                    if path.segs.first().is_some_and(|s| !matches!(s, Seg::Move(..))) {
                        return Err(SheetError::Invalid("a path starts with Move"));
                    }
                },
                Op::Text {
                    x, y, size, text, paint, ..
                } => {
                    if !(finite(*x) && finite(*y) && finite(*size) && *size > 0.0) {
                        return Err(SheetError::Invalid("text position or size"));
                    }
                    if text.len() > MAX_TEXT_BYTES {
                        return Err(SheetError::Invalid("text run too long"));
                    }
                    if !paint_valid(paint) {
                        return Err(SheetError::Invalid("alpha outside 0..=1"));
                    }
                },
            }
        }
        if segments > MAX_SEGMENTS {
            return Err(SheetError::Invalid("too many path segments"));
        }
        Ok(())
    }

    /// Version byte, then CBOR.
    pub fn encode(&self) -> Result<Vec<u8>, SheetError> {
        self.validate()?;
        let mut out = vec![SHEET_VERSION];
        ciborium::into_writer(self, &mut out).map_err(|e| SheetError::Decode(e.to_string()))?;
        if out.len() > MAX_PAYLOAD_BYTES {
            return Err(SheetError::TooLarge(out.len()));
        }
        Ok(out)
    }

    /// The inverse of [`Self::encode`]; validates what it decodes.
    pub fn decode(bytes: &[u8]) -> Result<Self, SheetError> {
        if bytes.len() > MAX_PAYLOAD_BYTES {
            return Err(SheetError::TooLarge(bytes.len()));
        }
        let (version, body) = bytes.split_first().ok_or(SheetError::Empty)?;
        if *version != SHEET_VERSION {
            return Err(SheetError::Version(*version));
        }
        let sheet: Self =
            ciborium::from_reader(body).map_err(|e| SheetError::Decode(e.to_string()))?;
        sheet.validate()?;
        Ok(sheet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> VectorSheet {
        VectorSheet {
            width: 100.0,
            height: 40.0,
            ops: vec![
                Op::Fill {
                    // A ring: two subpaths, even-odd leaves the counter empty.
                    path: Path {
                        segs: [Path::rect(0.0, 0.0, 20.0, 20.0).segs, Path::rect(5.0, 5.0, 10.0, 10.0).segs]
                            .concat(),
                    },
                    rule: FillRule::EvenOdd,
                    paint: Paint::new(Role::Ink),
                },
                Op::Stroke {
                    path: Path { segs: vec![Seg::Move(0.0, 30.0), Seg::Line(100.0, 30.0)] },
                    width: 1.0,
                    cap: Cap::Butt,
                    join: Join::Miter,
                    paint: Paint::with_alpha(Role::Muted, 0.5),
                },
                Op::Text {
                    x: 4.0,
                    y: 2.0,
                    size: 12.0,
                    text: "Dm7".into(),
                    anchor: Anchor::Start,
                    face: Face::Reading,
                    paint: Paint::new(Role::Accent),
                },
            ],
        }
    }

    #[test]
    fn round_trips_with_a_version_byte() {
        let bytes = sample().encode().unwrap();
        assert_eq!(bytes[0], SHEET_VERSION);
        assert_eq!(VectorSheet::decode(&bytes).unwrap(), sample());
    }

    #[test]
    fn compound_paths_keep_every_subpath() {
        let decoded = VectorSheet::decode(&sample().encode().unwrap()).unwrap();
        let Op::Fill { path, rule, .. } = &decoded.ops[0] else { panic!() };
        assert_eq!(*rule, FillRule::EvenOdd);
        assert_eq!(path.segs.iter().filter(|s| matches!(s, Seg::Move(..))).count(), 2);
    }

    #[test]
    fn refuses_unknown_versions_and_bad_numbers() {
        let mut bytes = sample().encode().unwrap();
        bytes[0] = 9;
        assert_eq!(VectorSheet::decode(&bytes), Err(SheetError::Version(9)));
        let mut bad = sample();
        bad.width = f32::NAN;
        assert!(matches!(bad.encode(), Err(SheetError::Invalid(_))));
        let mut bad = sample();
        if let Op::Stroke { paint, .. } = &mut bad.ops[1] {
            paint.alpha = 2.0;
        }
        assert!(matches!(bad.validate(), Err(SheetError::Invalid(_))));
    }

    #[test]
    fn refuses_oversize_and_garbage() {
        assert_eq!(VectorSheet::decode(&[]), Err(SheetError::Empty));
        assert!(matches!(
            VectorSheet::decode(&vec![1u8; MAX_PAYLOAD_BYTES + 1]),
            Err(SheetError::TooLarge(_))
        ));
        assert!(matches!(VectorSheet::decode(&[1, 0xff, 0xff]), Err(SheetError::Decode(_))));
        let mut long = sample();
        if let Op::Text { text, .. } = &mut long.ops[2] {
            *text = "x".repeat(MAX_TEXT_BYTES + 1);
        }
        assert!(matches!(long.validate(), Err(SheetError::Invalid(_))));
    }

    #[test]
    fn a_path_must_start_with_move() {
        let mut bad = sample();
        if let Op::Fill { path, .. } = &mut bad.ops[0] {
            path.segs.insert(0, Seg::Line(1.0, 1.0));
        }
        assert!(matches!(bad.validate(), Err(SheetError::Invalid(_))));
    }
}
