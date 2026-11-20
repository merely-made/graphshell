// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

use super::*;

/// A title's line height, px; it is centred in its card.
const TITLE_LINE: f32 = 16.0;
// The title's family names Tabard's interface role. A host publishes the
// role through `BoardText::set_font_roles`; until it does, the fallback is
// the face that renders: Plex Sans where the host lends it (Graphshell),
// else whatever the generic sans-serif resolves to among the lent faces.
pub(super) const TITLE_SHEET: &str = "\
    .board-title { position: absolute; margin: 0; padding: 0 6px; box-sizing: border-box; \
      font-family: var(--tabard-font-interface, IBM Plex Sans), sans-serif; font-size: 13px; line-height: 16px; \
      color: #f0dfb8; text-align: center; white-space: nowrap; overflow: hidden; }";

impl BoardText {
    pub fn new() -> Self {
        Self {
            text: genet_livery::TextSystem::new(),
            generation: 0,
            font_roles: String::new(),
        }
    }

    /// Declarations for the document's root custom properties, such as
    /// `--tabard-font-interface: "IBM Plex Sans";`, so the titles' `var()`
    /// resolves. Empty (the default) leaves the sheets as authored; a changed
    /// value applies on the next paint.
    pub fn set_font_roles(&mut self, declarations: &str) {
        self.font_roles = declarations.to_owned();
    }

    /// `sheet` under the host's root declarations. Each label is its own
    /// document root (no parent element), so `:root` reaches them.
    pub(crate) fn rooted(&self, sheet: &str) -> String {
        if self.font_roles.is_empty() {
            sheet.to_owned()
        } else {
            format!(":root {{ {} }}\n{sheet}", self.font_roles)
        }
    }

    /// Register one face, by its bytes.
    pub fn register_font(&mut self, bytes: Vec<u8>) {
        self.text.register_font_bytes(bytes);
    }

    /// Lay the titles out over their card rectangles and paint them.
    pub(super) fn titles<'a>(
        &mut self,
        cards: impl Iterator<Item = (&'a BoardCard, BoardRect)>,
        width: u32,
        height: u32,
    ) -> Option<genet_livery::LiveryPaintList> {
        let visible: Vec<_> = cards
            .filter(|(_, rect)| title_intersects_viewport(*rect, width, height))
            .map(|(card, rect)| (card.id.clone(), card.labelled(), rect))
            .collect();
        self.labels(&visible, width, height, TITLE_SHEET)
    }

    /// Shared label shaping for independent surfaces; appearance belongs to the host.
    pub(crate) fn labels(
        &mut self,
        cards: &[(String, String, BoardRect)],
        width: u32,
        height: u32,
        sheet: &str,
    ) -> Option<genet_livery::LiveryPaintList> {
        use layout_dom_api::{LayoutDom, LayoutDomMut};
        if cards.iter().all(|(_, title, _)| title.is_empty()) {
            return None;
        }
        let mut dom = genet_scripted_dom::ScriptedDom::new();
        let root = dom.document();
        for (_, title, rect) in cards.iter().filter(|(_, title, _)| !title.is_empty()) {
            let line = dom.create_element(qual("div"));
            dom.set_attribute(line, qual("class"), "board-title");
            dom.set_attribute(
                line,
                qual("style"),
                &format!(
                    "left: {:.1}px; top: {:.1}px; width: {:.1}px;",
                    rect.x,
                    title_top(*rect),
                    rect.width
                ),
            );
            let words = dom.create_text(title);
            dom.append_child(line, words);
            dom.append_child(root, line);
        }
        let sheet = self.rooted(sheet);
        let styles = genet_livery::resolve_styles(
            &dom,
            &genet_livery::StyleSet::cambium(&[sheet.as_str()]),
            &genet_livery::Device::screen(width as f32, height as f32),
            &genet_livery::InteractionStates::default(),
        );
        let (styles, layout) = genet_livery::layout_with_text_system(
            &dom,
            &styles,
            width as f32,
            height as f32,
            genet_livery::ViewportSizes::uniform(width as f32, height as f32),
            &mut self.text,
            &std::collections::HashMap::new(),
        )
        .ok()?;
        self.generation += 1;
        Some(genet_livery::emit_paint_list_with_text_system(
            &dom,
            &styles,
            &layout,
            DeviceIntSize::new(width as i32, height as i32),
            self.generation,
            &mut self.text,
        ))
    }
}

fn title_top(card: BoardRect) -> f32 {
    card.y + (card.height - TITLE_LINE) * 0.5
}

pub(super) fn title_intersects_viewport(card: BoardRect, width: u32, height: u32) -> bool {
    let top = title_top(card);
    // The title is clipped to its own line box, which can extend beyond a
    // short card. Its border-box width is at least its two 6 px paddings.
    let right = card.x + card.width.max(12.0);
    let bottom = top + TITLE_LINE;
    // Keep edge cases where the authored CSS rounds a position or width to
    // one decimal. Let Livery decide the paint for nonfinite input as before.
    let rounding = 0.1;
    ![card.x, top, right, bottom, card.width]
        .iter()
        .all(|v| v.is_finite())
        || (card.x < width as f32 + rounding
            && right > -rounding
            && top < height as f32 + rounding
            && bottom > -rounding)
}
