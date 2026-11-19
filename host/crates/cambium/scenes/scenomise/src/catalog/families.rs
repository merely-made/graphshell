// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

impl Family {
    pub fn id(self) -> &'static str {
        match self {
            Self::Spiral => "spiral",
            Self::Grid => "grid",
            Self::Geographic => "geographic",
            Self::Hulls => "hulls",
            Self::Stack => "stack",
            Self::Penrose => "penrose",
            Self::LSystem => "lsystem",
            Self::Timeline => "timeline",
            Self::Kanban => "kanban",
            Self::Embedded => "embedded",
            Self::Radial => "radial",
            Self::Partition => "partition",
            Self::Sets => "sets",
            Self::Mosaic => "mosaic",
            Self::Crystal => "crystal",
            Self::Table => "table",
        }
    }

    /// Older ids that saved recipes carry.
    pub fn aliases(self) -> &'static [&'static str] {
        match self {
            Self::Grid => &[GRID_ARRANGEMENT_ID, "grid.ordered"],
            Self::Geographic => &[SCATTER_ARRANGEMENT_ID],
            _ => &[],
        }
    }

    /// The family an authored id or alias names, if it is built in.
    pub fn resolve(id: &str) -> Option<Self> {
        FAMILIES
            .into_iter()
            .find(|family| family.id() == id || family.aliases().contains(&id))
    }

    /// Placed forms this family can realize from disclosed relationships.
    pub fn placed_forms(self, options: &BTreeMap<String, String>) -> Vec<sceno::RelationForm> {
        use sceno::RelationForm;
        match self {
            Self::Grid | Self::Spiral | Self::Mosaic => vec![RelationForm::Order],
            Self::Crystal => vec![RelationForm::Adjacency],
            Self::Partition if options.get("form").is_none_or(|form| form == "tiles") => {
                vec![RelationForm::Containment]
            },
            _ => Vec::new(),
        }
    }

    /// Frame kinds realized by this family, shared by validation and pickers.
    pub fn frames(self) -> &'static [sceno::FrameKind] {
        use sceno::FrameKind::*;
        match self {
            Self::Partition | Self::Sets | Self::Hulls => &[Bounded],
            Self::Grid => &[Unbounded, Nested, Flow],
            Self::Table => &[Flow, Unbounded],
            _ => &[Unbounded],
        }
    }

    pub fn channels(self) -> ChannelUse {
        match self {
            Self::Grid => ChannelUse::Cells,
            Self::Geographic | Self::Hulls => ChannelUse::Coordinate,
            Self::Embedded => ChannelUse::Embedding,
            Self::Timeline | Self::Radial => ChannelUse::NumericAxis,
            Self::Stack => ChannelUse::IntegerAxis,
            Self::Kanban | Self::Partition | Self::Sets | Self::Mosaic => {
                ChannelUse::CategoricalAxis
            },
            Self::Spiral | Self::Penrose | Self::LSystem | Self::Crystal | Self::Table => {
                ChannelUse::Order
            },
        }
    }

    /// The extent this family imposes on every item (primitive
    /// representations plan, M1): the families that write the footprint.
    pub fn extent(self) -> Extent {
        match self {
            Self::Partition | Self::Mosaic | Self::Crystal | Self::Table => Extent::Assigned,
            _ => Extent::Measured,
        }
    }

    /// The mark the family draws its items as when a recipe names none; the
    /// class registry decides for the rest.
    pub fn default_mark(self) -> Option<scenograph::presentation::NodeMark> {
        match self {
            Self::Table => Some(scenograph::presentation::NodeMark::row()),
            _ => None,
        }
    }

    /// What the family advertises to pickers. Every built-in family is closed
    /// form, so every one replays.
    pub fn capability(self) -> SolverCapability {
        let (name, description, tags): (&str, &str, &[&str]) = match self {
            Self::Spiral => ("Spiral", "Items wind outward in order.", &["organic"]),
            Self::Grid => ("Grid", "Items take ranked cells.", &["spatial-memory"]),
            Self::Geographic => (
                "Geographic",
                "Items sit at their coordinates.",
                &["spatial-memory"],
            ),
            Self::Hulls => (
                "Hulls",
                "Each item owns its nearest region.",
                &["spatial-memory"],
            ),
            Self::Stack => (
                "Stack",
                "Items layer by an integral rank.",
                &["hierarchical"],
            ),
            Self::Penrose => (
                "Penrose",
                "Items take aperiodic tiling vertices in order.",
                &["organic"],
            ),
            Self::LSystem => (
                "L-system",
                "Items follow a fractal path in order.",
                &["organic"],
            ),
            Self::Timeline => (
                "Timeline",
                "Items place along a numeric axis.",
                &["time-axis"],
            ),
            Self::Kanban => ("Kanban", "Items sort into named columns.", &[]),
            Self::Embedded => (
                "Embedded",
                "Items sit at computed 2D coordinates.",
                &["spatial-memory"],
            ),
            Self::Radial => (
                "Radial",
                "Items ring outward by a numeric index.",
                &["hierarchical"],
            ),
            Self::Partition => (
                "Partition",
                "Items share a measured whole, by area or by angle.",
                &["hierarchical"],
            ),
            Self::Sets => (
                "Sets",
                "Items sit in the overlap of the sets they belong to.",
                &["spatial-memory"],
            ),
            Self::Crystal => (
                "Crystal",
                "Pinned tiles pack their immediate links as shared borders.",
                &["spatial-memory", "adjacency"],
            ),
            Self::Mosaic => (
                "Mosaic",
                "Items tile by weight in groups, and never reflow.",
                &["spatial-memory", "hierarchical"],
            ),
            Self::Table => (
                "Table",
                "One row per item in reading order, one column per field.",
                &["tabular"],
            ),
        };
        let mut capability = SolverCapability::new(self.id(), name);
        capability.description = Some(description.into());
        capability.requires = match self.channels() {
            ChannelUse::Embedding => vec![Disclosure::Embedding],
            ChannelUse::NumericAxis | ChannelUse::IntegerAxis | ChannelUse::CategoricalAxis => {
                vec![Disclosure::Axis]
            },
            ChannelUse::Cells | ChannelUse::Coordinate | ChannelUse::Order => Vec::new(),
        };
        capability.tags = tags.iter().map(|tag| (*tag).to_owned()).collect();
        capability.options = self.options();
        capability.frames = self.frames().to_vec();
        capability.placed_forms = self.placed_forms(&BTreeMap::new());
        capability
    }

    /// Build the typed arrangement from authored options and measured items.
    pub fn arrangement(
        self,
        options: &BTreeMap<String, String>,
        measure: &Measure,
    ) -> Result<Arrangement, Vec<OptionIssue>> {
        let mut read = Options::new(options, self.options());
        let arrangement = self.read_arrangement(&mut read, measure);
        read.finish(self)?;
        Ok(arrangement)
    }

    /// What each option is when the author leaves it out, for these items:
    /// the declaration's measured defaults resolved to numbers (SE43).
    pub fn resolved_defaults(self, measure: &Measure) -> BTreeMap<String, String> {
        let empty = BTreeMap::new();
        let mut read = Options::new(&empty, self.options());
        let _ = self.read_arrangement(&mut read, measure);
        read.resolved
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect()
    }

    pub(super) fn read_arrangement(self, read: &mut Options, measure: &Measure) -> Arrangement {
        match self {
            Self::Spiral => {
                let default = Spiral::default();
                Arrangement::Spiral(Spiral {
                    center: Vec2::ZERO,
                    spacing: measure.spacing,
                    angle_radians: read.finite("angle_radians", default.angle_radians),
                    curve: read.choice("curve", default.curve, SPIRAL_CURVES),
                })
            },
            Self::Grid => Arrangement::Grid(Grid {
                origin: Vec2::ZERO,
                cell: Vec2::new(
                    read.positive("cell_width", measure.largest.w),
                    read.positive("cell_height", measure.largest.h),
                ),
                columns: read.count("columns", measure.side() as u32),
                gap: measure.spacing,
            }),
            Self::Geographic => Arrangement::Geographic(Geographic {
                origin: Vec2::ZERO,
                units_per_coordinate: read.positive("units_per_coordinate", measure.spacing),
                invert_y: read.flag("invert_y", false),
            }),
            Self::Hulls => {
                let units = read.positive("units_per_coordinate", measure.spacing);
                let invert_y = read.flag("invert_y", false);
                Arrangement::Hulls(Hulls {
                    origin: Vec2::ZERO,
                    units_per_coordinate: units,
                    invert_y,
                    bounds: hull_bounds(measure, units, invert_y),
                })
            },
            Self::Stack => Arrangement::Stack(Stack {
                layer_gap: read.positive("layer_gap", measure.pitch_h()),
                row_gap: read.positive("row_gap", measure.pitch_w()),
                center: Vec2::ZERO,
            }),
            Self::Penrose => Arrangement::Penrose(Penrose {
                variant: read.choice("variant", PenroseVariant::default(), PENROSE_VARIANTS),
                subdivision_count: match read.depth("subdivisions") {
                    Some(depth) => SubdivisionCount::Explicit(depth),
                    None => SubdivisionCount::Auto,
                },
                unused_vertices: read.choice(
                    "unused_vertices",
                    UnusedVertexPolicy::default(),
                    PENROSE_UNUSED_VERTICES,
                ),
                center: Vec2::ZERO,
                // The tiling fills a disc: give each item a pitch-sized share of it.
                tile_scale: read.positive(
                    "tile_scale",
                    measure.pitch() * (measure.count.max(1) as f32 / std::f32::consts::PI).sqrt(),
                ),
            }),
            Self::LSystem => Arrangement::LSystem(LSystem {
                grammar: read.choice("grammar", LSystemGrammar::default(), LSYSTEM_GRAMMARS),
                iteration_depth: match read.depth("depth") {
                    Some(depth) => IterationDepth::Explicit(depth),
                    None => IterationDepth::Auto,
                },
                origin: Vec2::ZERO,
                // The path fills a square with a pitch between neighbors.
                size: read.positive("size", measure.side() * measure.pitch()),
                rotation: read.finite("rotation", 0.0),
                reverse_order: read.flag("reverse_order", false),
            }),
            Self::Timeline => Arrangement::Timeline(Timeline {
                origin: Vec2::ZERO,
                axis_length: read.positive(
                    "axis_length",
                    measure.count.max(1) as f32 * measure.pitch_w(),
                ),
                row_gap: read.positive("row_gap", measure.pitch_h()),
                fallback: read.choice("fallback", TimelineFallback::default(), TIMELINE_FALLBACKS),
            }),
            Self::Kanban => Arrangement::Kanban(Kanban {
                origin: Vec2::ZERO,
                column_gap: read.positive("column_gap", measure.pitch_w()),
                row_gap: read.positive("row_gap", measure.pitch_h()),
                column_order: read.list("column_order"),
                include_other_column: read.flag("include_other_column", true),
            }),
            Self::Embedded => Arrangement::Embedded(Embedded {
                origin: Vec2::ZERO,
                scale: read.positive("scale", measure.spacing),
                rotation: read.finite("rotation", 0.0),
                fallback: read.choice(
                    "fallback",
                    EmbeddingFallback::default(),
                    EMBEDDING_FALLBACKS,
                ),
            }),
            Self::Radial => Arrangement::Radial(Radial {
                center: Vec2::ZERO,
                ring_spacing: read.positive("ring_spacing", measure.pitch()),
                angular_policy: read.choice(
                    "angular_policy",
                    RadialAngularPolicy::default(),
                    RADIAL_ANGULAR_POLICIES,
                ),
                rotation_offset: read.finite("rotation_offset", 0.0),
                unreachable_policy: read.choice(
                    "unreachable_policy",
                    RadialUnreachablePolicy::default(),
                    RADIAL_UNREACHABLE_POLICIES,
                ),
            }),
            Self::Partition => Arrangement::Partition(Partition {
                bounds: area(read, measure),
                whole: Some(read.finite("whole", 0.0)).filter(|whole| *whole > 0.0),
                form: read.choice("form", PartitionForm::default(), PARTITION_FORMS),
                gap: read.finite("gap", Partition::default().gap).max(0.0),
            }),
            Self::Sets => Arrangement::Setscape(Setscape {
                bounds: area(read, measure),
                sets: read.list("sets"),
            }),
            Self::Crystal => {
                let smallest_side =
                    read.positive("smallest_side", Crystal::default().smallest_side);
                Arrangement::Crystal(Crystal {
                    origin: Vec2::ZERO,
                    tile: Size2::new(
                        measure.largest.w.max(smallest_side),
                        measure.largest.h.max(smallest_side),
                    ),
                    smallest_side,
                    spacing: measure.spacing,
                })
            },
            Self::Mosaic => {
                let default = Mosaic::default();
                Arrangement::Mosaic(Mosaic {
                    origin: Vec2::ZERO,
                    unit_area: read.positive("unit_area", measure.pitch() * measure.pitch()),
                    aspect: read.positive("aspect", default.aspect),
                    gap: read.finite("gap", default.gap).max(0.0),
                    header: read.finite("header", default.header).max(0.0),
                    group_order: read.list("group_order"),
                })
            },
            Self::Table => {
                let column = read.positive("column_width", measure.largest.w);
                let row_height = read.positive("row_height", measure.largest.h);
                Arrangement::Table(Table {
                    origin: Vec2::ZERO,
                    columns: vec![column; measure.columns.max(1)],
                    row_height,
                    header: read.finite("header", row_height).max(0.0),
                })
            },
        }
    }
}
