// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Family {
    /// The options this family reads, declared as data: one declaration for
    /// refusal and for an editor's rows (Scenograph editor plan, ruling B, SE7).
    pub fn options(self) -> Vec<OptionSpec> {
        let finite = |key: &str, label: &str, default: f32| {
            OptionSpec::new(
                key,
                label,
                OptionKind::Finite,
                OptionDefault::Value(default.to_string()),
            )
        };
        let measured = |key: &str, label: &str, kind: OptionKind, words: &str| {
            OptionSpec::new(key, label, kind, OptionDefault::Measured(words.to_owned()))
        };
        let flag = |key: &str, label: &str, default: bool| {
            OptionSpec::new(
                key,
                label,
                OptionKind::Flag,
                OptionDefault::Value(default.to_string()),
            )
        };
        fn choice<T: PartialEq + Default>(
            key: &str,
            label: &str,
            table: &[(&str, T)],
        ) -> OptionSpec {
            let default = T::default();
            let name = table
                .iter()
                .find(|(_, value)| *value == default)
                .map(|(name, _)| (*name).to_owned())
                .expect("a choice table names its default");
            OptionSpec::new(
                key,
                label,
                OptionKind::Choice {
                    names: table.iter().map(|(name, _)| (*name).to_owned()).collect(),
                },
                OptionDefault::Value(name),
            )
        }
        let auto_depth = |key: &str, label: &str| {
            OptionSpec::new(key, label, OptionKind::Depth, OptionDefault::Auto)
        };
        let spacing = "the recipe's spacing";
        match self {
            Self::Spiral => vec![
                finite(
                    "angle_radians",
                    "Turn per item",
                    Spiral::default().angle_radians,
                ),
                choice("curve", "Curve", SPIRAL_CURVES),
            ],
            Self::Grid => vec![
                measured(
                    "cell_width",
                    "Cell width",
                    OptionKind::Positive,
                    "the largest item's width",
                ),
                measured(
                    "cell_height",
                    "Cell height",
                    OptionKind::Positive,
                    "the largest item's height",
                ),
                measured(
                    "columns",
                    "Columns",
                    OptionKind::Count,
                    "the side of the square that holds every item",
                ),
            ],
            Self::Geographic | Self::Hulls => vec![
                measured(
                    "units_per_coordinate",
                    "Units per coordinate",
                    OptionKind::Positive,
                    spacing,
                ),
                flag("invert_y", "Invert y", false),
            ],
            Self::Stack => vec![
                measured(
                    "layer_gap",
                    "Layer gap",
                    OptionKind::Positive,
                    "the largest item's height plus spacing",
                ),
                measured(
                    "row_gap",
                    "Row gap",
                    OptionKind::Positive,
                    "the largest item's width plus spacing",
                ),
            ],
            Self::Penrose => vec![
                choice("variant", "Variant", PENROSE_VARIANTS),
                auto_depth("subdivisions", "Subdivisions"),
                choice(
                    "unused_vertices",
                    "Unused vertices",
                    PENROSE_UNUSED_VERTICES,
                ),
                measured(
                    "tile_scale",
                    "Tile scale",
                    OptionKind::Positive,
                    "an item's pitch, scaled so the tiling's disc holds every item",
                ),
            ],
            Self::LSystem => vec![
                choice("grammar", "Grammar", LSYSTEM_GRAMMARS),
                auto_depth("depth", "Depth"),
                measured(
                    "size",
                    "Size",
                    OptionKind::Positive,
                    "the side of the square that holds every item, times an item's pitch",
                ),
                finite("rotation", "Rotation", 0.0),
                flag("reverse_order", "Reverse order", false),
            ],
            Self::Timeline => vec![
                measured(
                    "axis_length",
                    "Axis length",
                    OptionKind::Positive,
                    "the item count times an item's width plus spacing",
                ),
                measured(
                    "row_gap",
                    "Row gap",
                    OptionKind::Positive,
                    "the largest item's height plus spacing",
                ),
                choice("fallback", "Fallback", TIMELINE_FALLBACKS),
            ],
            Self::Kanban => vec![
                measured(
                    "column_gap",
                    "Column gap",
                    OptionKind::Positive,
                    "the largest item's width plus spacing",
                ),
                measured(
                    "row_gap",
                    "Row gap",
                    OptionKind::Positive,
                    "the largest item's height plus spacing",
                ),
                OptionSpec::new(
                    "column_order",
                    "Column order",
                    OptionKind::List,
                    OptionDefault::Empty,
                ),
                flag("include_other_column", "Include an other column", true),
            ],
            Self::Embedded => vec![
                measured("scale", "Scale", OptionKind::Positive, spacing),
                finite("rotation", "Rotation", 0.0),
                choice("fallback", "Fallback", EMBEDDING_FALLBACKS),
            ],
            Self::Radial => vec![
                measured(
                    "ring_spacing",
                    "Ring spacing",
                    OptionKind::Positive,
                    "an item's pitch",
                ),
                choice("angular_policy", "Angular policy", RADIAL_ANGULAR_POLICIES),
                finite("rotation_offset", "Rotation offset", 0.0),
                choice(
                    "unreachable_policy",
                    "Unreachable items",
                    RADIAL_UNREACHABLE_POLICIES,
                ),
            ],
            Self::Partition => vec![
                measured("width", "Width", OptionKind::Positive, AREA),
                measured("height", "Height", OptionKind::Positive, AREA),
                finite("whole", "Measured whole (0: the items' total)", 0.0),
                choice("form", "Form", PARTITION_FORMS),
                finite("gap", "Gap", Partition::default().gap),
            ],
            Self::Sets => vec![
                measured("width", "Width", OptionKind::Positive, AREA),
                measured("height", "Height", OptionKind::Positive, AREA),
                OptionSpec::new("sets", "Sets", OptionKind::List, OptionDefault::Empty),
            ],
            Self::Crystal => vec![OptionSpec::new(
                "smallest_side",
                "Smallest tile side",
                OptionKind::Positive,
                OptionDefault::Value(Crystal::default().smallest_side.to_string()),
            )],
            Self::Mosaic => vec![
                measured(
                    "unit_area",
                    "Area per unit of weight",
                    OptionKind::Positive,
                    "the largest item's pitch, squared",
                ),
                OptionSpec::new(
                    "aspect",
                    "Aspect",
                    OptionKind::Positive,
                    OptionDefault::Value(Mosaic::default().aspect.to_string()),
                ),
                finite("gap", "Gap", Mosaic::default().gap),
                finite("header", "Group label band", Mosaic::default().header),
                OptionSpec::new(
                    "group_order",
                    "Group order",
                    OptionKind::List,
                    OptionDefault::Empty,
                ),
            ],
            Self::Table => vec![
                measured(
                    "column_width",
                    "Column width",
                    OptionKind::Positive,
                    "the largest item's width",
                ),
                measured(
                    "row_height",
                    "Row height",
                    OptionKind::Positive,
                    "the largest item's height",
                ),
                measured(
                    "header",
                    "Header band",
                    OptionKind::Finite,
                    "one row height",
                ),
            ],
        }
    }
}
