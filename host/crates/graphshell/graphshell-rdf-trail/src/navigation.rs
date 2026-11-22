// Copyright 2026 Mark Alan Boykin
// SPDX-License-Identifier: MPL-2.0

use hodograph::{
    BindingSnapshot, EntrySnapshot, OwnerSnapshot, StemmaSnapshot, TransitionRecord, VisitSnapshot,
};
use kernel::graph::NodeHistoryOwner;

use super::*;

type Navigation = StemmaSnapshot<String, String, NodeHistoryOwner, ()>;

pub(super) fn write(trail: &Navigation, out: &mut Writer) -> Result<()> {
    for (index, entry) in trail.entries.iter().enumerate() {
        let row = out.row("entry", index, "Page");
        out.set(&row, "entryKey", Literal::from(entry.key.as_str()));
        out.set(&row, "address", Literal::from(entry.payload.as_str()));
        out.set(&row, "firstSeenMs", unsigned(entry.first_seen_at_ms));
        out.set(&row, "lastSeenMs", unsigned(entry.last_seen_at_ms));
        out.set(&row, "visitCount", unsigned(entry.visit_count));
        out.set(&row, "privacy", enum_term(entry.privacy)?);
    }

    let mut owner_for_visit = HashMap::new();
    for (index, owner) in trail.owners.iter().enumerate() {
        let row = out.row("owner", index, "HistoryOwner");
        let NodeHistoryOwner::Node(id) = &owner.identity;
        out.set(
            &row,
            "surface",
            surface(Uuid::parse_str(id).map_err(|e| e.to_string())?),
        );
        for (predicate, table, reference) in [
            ("originVisit", "visit", owner.origin),
            ("currentVisit", "visit", owner.current),
            ("creator", "owner", owner.creator),
            ("pendingParent", "visit", owner.pending_origin_parent),
        ] {
            if let Some(reference) = reference {
                out.set(&row, predicate, out.id(table, reference));
            }
        }
        for visit in &owner.owned_visits {
            owner_for_visit.insert(*visit, id);
            out.set(&row, "ownsVisit", out.id("visit", *visit));
        }
    }

    let child_orders: HashMap<_, _> = trail
        .visits
        .iter()
        .flat_map(|parent| {
            parent
                .children
                .iter()
                .copied()
                .enumerate()
                .map(|(order, child)| (child, order))
        })
        .collect();
    let mut binding_index = 0;
    for (index, visit) in trail.visits.iter().enumerate() {
        let row = out.row("visit", index, "Visit");
        out.set(&row, "page", out.id("entry", visit.entry));
        out.set(&row, "timestampMs", unsigned(visit.created_at_ms));
        if let Some(id) = owner_for_visit.get(&index) {
            out.set(
                &row,
                "toSurface",
                surface(Uuid::parse_str(id).map_err(|e| e.to_string())?),
            );
        }
        if let Some(parent) = visit.parent {
            out.set(&row, "fromVisit", out.id("visit", parent));
            let order = child_orders
                .get(&index)
                .ok_or("visit missing from its parent's children")?;
            out.set(&row, "childOrder", unsigned(*order as u64));
        }
        if let Some(inbound) = visit.inbound {
            out.set(&row, "trigger", enum_term(inbound.kind)?);
            out.set(&row, "transitionMs", unsigned(inbound.at_ms));
        }
        for binding in &visit.bindings {
            let id = out.row("binding", binding_index, "VisitBinding");
            binding_index += 1;
            out.set(&row, "binding", id.clone());
            out.set(&id, "owner", out.id("owner", binding.owner));
            out.set(&id, "lastAccessedMs", unsigned(binding.last_accessed_at_ms));
            if let Some(child) = binding.forward_child {
                out.set(&id, "forwardVisit", out.id("visit", child));
            }
        }
    }
    Ok(())
}

fn optional_index(
    source: &Reader,
    row: &NamedNode,
    prop: &str,
    table: &str,
) -> Result<Option<usize>> {
    source
        .optional(row, vocab(prop).as_str())?
        .map(|value| source.index(value, table))
        .transpose()
}

fn indexes(source: &Reader, row: &NamedNode, prop: &str, table: &str) -> Result<Vec<usize>> {
    let mut numbers = source
        .values(row, vocab(prop).as_str())
        .iter()
        .map(|term| source.index(term, table))
        .collect::<Result<Vec<_>>>()?;
    numbers.sort_unstable();
    Ok(numbers)
}

pub(super) fn read(source: &Reader) -> Result<Navigation> {
    let entries = source
        .rows("entry", "Page")?
        .into_iter()
        .map(|row| {
            Ok(EntrySnapshot {
                key: source.text(row, "entryKey")?,
                payload: source.text(row, "address")?,
                first_seen_at_ms: source.uint(row, "firstSeenMs")?,
                last_seen_at_ms: source.uint(row, "lastSeenMs")?,
                visit_count: source.uint(row, "visitCount")?,
                privacy: source.enumeration(row, "privacy")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let owners = source
        .rows("owner", "HistoryOwner")?
        .into_iter()
        .map(|row| {
            let id = surface_id(as_named(source.get(row, "surface")?)?)?;
            Ok(OwnerSnapshot {
                identity: NodeHistoryOwner::Node(id.to_string()),
                origin: optional_index(source, row, "originVisit", "visit")?,
                current: optional_index(source, row, "currentVisit", "visit")?,
                creator: optional_index(source, row, "creator", "owner")?,
                pending_origin_parent: optional_index(source, row, "pendingParent", "visit")?,
                owned_visits: indexes(source, row, "ownsVisit", "visit")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let binding_rows = source.rows("binding", "VisitBinding")?;
    let mut visits = source
        .rows("visit", "Visit")?
        .into_iter()
        .map(|row| {
            let bindings = indexes(source, row, "binding", "binding")?
                .into_iter()
                .map(|index| {
                    let row = binding_rows.get(index).ok_or("missing visit binding")?;
                    Ok(BindingSnapshot {
                        owner: source.index(source.get(row, "owner")?, "owner")?,
                        forward_child: optional_index(source, row, "forwardVisit", "visit")?,
                        last_accessed_at_ms: source.uint(row, "lastAccessedMs")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            let inbound = if source.optional(row, vocab("trigger").as_str())?.is_some() {
                Some(TransitionRecord {
                    kind: source.enumeration(row, "trigger")?,
                    at_ms: source.uint(row, "transitionMs")?,
                })
            } else {
                None
            };
            Ok(VisitSnapshot {
                entry: source.index(source.get(row, "page")?, "entry")?,
                parent: optional_index(source, row, "fromVisit", "visit")?,
                children: Vec::new(),
                created_at_ms: source.uint(row, "timestampMs")?,
                context: (),
                inbound,
                bindings,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    let rows = source.rows("visit", "Visit")?;
    let mut by_parent = BTreeMap::<usize, Vec<(u64, usize)>>::new();
    let mut surfaces = HashMap::new();
    for owner in &owners {
        let NodeHistoryOwner::Node(id) = &owner.identity;
        let id = surface(Uuid::parse_str(id).map_err(|err| err.to_string())?);
        for visit in &owner.owned_visits {
            if surfaces.insert(*visit, id.clone()).is_some() {
                return Err("visit has multiple owners".into());
            }
        }
    }
    for (child, row) in rows.into_iter().enumerate() {
        let address = source.optional(row, vocab("toSurface").as_str())?;
        let surface = address.map(as_named).transpose()?;
        if surface != surfaces.get(&child) {
            return Err("visit surface differs from its owner's surface".into());
        }
        if let Some(parent) = visits[child].parent {
            by_parent
                .entry(parent)
                .or_default()
                .push((source.uint(row, "childOrder")?, child));
        }
    }
    for (parent, mut children) in by_parent {
        let visit = visits.get_mut(parent).ok_or("missing visit parent")?;
        children.sort_unstable();
        for (index, (order, child)) in children.into_iter().enumerate() {
            if order != index as u64 {
                return Err("visit child order must be unique and contiguous".into());
            }
            visit.children.push(child);
        }
    }
    check_ancestry(visits.iter().map(|visit| visit.parent).collect())?;
    check_ancestry(owners.iter().map(|owner| owner.creator).collect())?;

    Ok(Navigation {
        entries,
        owners,
        visits,
    })
}

// Follow each ancestry chain at most once. Invalid references and cycles
// must fail before the data can become a navigable graph.
fn check_ancestry(parents: Vec<Option<usize>>) -> Result<()> {
    let mut visited = vec![None; parents.len()];
    for start in 0..parents.len() {
        let mut next = Some(start);
        while let Some(index) = next {
            let prior = visited.get_mut(index).ok_or("unknown ancestor")?;
            match *prior {
                Some(path) if path == start => return Err("cyclic trail ancestry".into()),
                Some(_) => break,
                None => *prior = Some(start),
            }
            next = parents[index];
        }
    }
    Ok(())
}
