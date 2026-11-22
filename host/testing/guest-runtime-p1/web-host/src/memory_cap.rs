// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! The memory cap for a guest on a JavaScript engine's wasm support.
//!
//! A guest's core module declares its own memory with no maximum, and the
//! engine offers no per-instance limit, so the cap is written into the module
//! before it is compiled: the memory section's maximum becomes the cap and
//! every other section is copied through byte for byte. The engine then
//! refuses `memory.grow` past it, as `StoreLimits` does natively.

use wasm_encoder::{MemorySection, MemoryType, Module, RawSection};
use wasmparser::{Parser, Payload};

/// `module` with every declared memory limited to `max_pages` pages. A module
/// with no memory section comes back unchanged. A memory that already has a
/// smaller maximum keeps it; one whose minimum exceeds the cap is an error,
/// since it could never be created within the limit.
pub fn limit_memory(module: &[u8], max_pages: u64) -> Result<Vec<u8>, String> {
    let mut out = Module::new();
    for payload in Parser::new(0).parse_all(module) {
        let payload = payload.map_err(|e| format!("not a core module: {e}"))?;
        if let Payload::MemorySection(reader) = &payload {
            let mut memories = MemorySection::new();
            for memory in reader.clone() {
                let memory = memory.map_err(|e| format!("bad memory section: {e}"))?;
                if memory.page_size_log2.is_some() {
                    return Err("a custom page size is not supported".into());
                }
                if memory.initial > max_pages {
                    return Err(format!(
                        "memory starts at {} pages, over the cap of {max_pages}",
                        memory.initial
                    ));
                }
                memories.memory(MemoryType {
                    minimum: memory.initial,
                    maximum: Some(memory.maximum.map_or(max_pages, |m| m.min(max_pages))),
                    memory64: memory.memory64,
                    shared: memory.shared,
                    page_size_log2: None,
                });
            }
            out.section(&memories);
        } else if let Some((id, range)) = payload.as_section() {
            out.section(&RawSection { id, data: &module[range.start as usize..range.end as usize] });
        }
    }
    Ok(out.finish())
}

#[cfg(test)]
mod tests {
    use super::*;
    use wasm_encoder::{
        CodeSection, ExportKind, ExportSection, Function, FunctionSection, Instruction, TypeSection,
        ValType,
    };

    const CAP_PAGES: u64 = 512;
    /// The id of the memory section in the binary format.
    const MEMORY_SECTION_ID: u8 = 5;

    /// A module with one function `f` returning 7 and a memory declared
    /// `memory`, or no memory when `None`.
    fn module(memory: Option<MemoryType>) -> Vec<u8> {
        let mut types = TypeSection::new();
        types.ty().function([], [ValType::I32]);
        let mut functions = FunctionSection::new();
        functions.function(0);
        let mut exports = ExportSection::new();
        exports.export("f", ExportKind::Func, 0);
        let mut code = CodeSection::new();
        let mut body = Function::new([]);
        body.instruction(&Instruction::I32Const(7));
        body.instruction(&Instruction::End);
        code.function(&body);
        let mut m = Module::new();
        m.section(&types);
        m.section(&functions);
        if let Some(memory) = memory {
            let mut memories = MemorySection::new();
            memories.memory(memory);
            m.section(&memories);
        }
        m.section(&exports);
        m.section(&code);
        m.finish()
    }

    fn unbounded(minimum: u64) -> MemoryType {
        MemoryType { minimum, maximum: None, memory64: false, shared: false, page_size_log2: None }
    }

    /// The limits of the module's memories, read back from the bytes.
    fn memories(module: &[u8]) -> Vec<(u64, Option<u64>)> {
        let mut found = Vec::new();
        for payload in Parser::new(0).parse_all(module) {
            if let Payload::MemorySection(reader) = payload.unwrap() {
                for memory in reader {
                    let memory = memory.unwrap();
                    found.push((memory.initial, memory.maximum));
                }
            }
        }
        found
    }

    /// Every section other than the memory section, as (id, bytes).
    fn other_sections(module: &[u8]) -> Vec<(u8, Vec<u8>)> {
        Parser::new(0)
            .parse_all(module)
            .filter_map(|p| p.unwrap().as_section())
            .filter(|(id, _)| *id != MEMORY_SECTION_ID)
            .map(|(id, range)| (id, module[range.start as usize..range.end as usize].to_vec()))
            .collect()
    }

    #[test]
    fn an_unbounded_memory_gets_the_cap() {
        let input = module(Some(unbounded(17)));
        let output = limit_memory(&input, CAP_PAGES).unwrap();
        assert_eq!(memories(&output), vec![(17, Some(CAP_PAGES))]);
    }

    #[test]
    fn the_result_is_a_valid_module_and_other_sections_are_unchanged() {
        let input = module(Some(unbounded(1)));
        let output = limit_memory(&input, CAP_PAGES).unwrap();
        wasmparser::Validator::new().validate_all(&output).expect("valid module");
        assert_eq!(other_sections(&input), other_sections(&output));
    }

    #[test]
    fn a_smaller_existing_maximum_is_kept() {
        let declared = MemoryType { maximum: Some(10), ..unbounded(1) };
        let output = limit_memory(&module(Some(declared)), CAP_PAGES).unwrap();
        assert_eq!(memories(&output), vec![(1, Some(10))]);
    }

    #[test]
    fn a_larger_existing_maximum_is_lowered() {
        let declared = MemoryType { maximum: Some(CAP_PAGES * 4), ..unbounded(1) };
        let output = limit_memory(&module(Some(declared)), CAP_PAGES).unwrap();
        assert_eq!(memories(&output), vec![(1, Some(CAP_PAGES))]);
    }

    #[test]
    fn a_module_without_memory_comes_back_unchanged() {
        let input = module(None);
        assert_eq!(limit_memory(&input, CAP_PAGES).unwrap(), input);
    }

    #[test]
    fn a_minimum_over_the_cap_is_an_error() {
        let error = limit_memory(&module(Some(unbounded(CAP_PAGES + 1))), CAP_PAGES).unwrap_err();
        assert!(error.contains("over the cap"), "{error}");
    }

    #[test]
    fn bytes_that_are_not_a_module_are_an_error() {
        assert!(limit_memory(b"not wasm", CAP_PAGES).is_err());
    }
}
