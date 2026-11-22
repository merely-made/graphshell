// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

//! Collecting the requests of one script run into a batch.
//!
//! A script asks for commands one at a time; a [`Batch`] admits each against
//! the offered set and its [`CommandLimits`]. The first request it refuses
//! voids the whole batch, even if the script caught the refusal and carried
//! on. A batch is data: collecting is neither permission nor execution.

use crate::Invocation;
use crate::menu::CommandSet;

/// The default cap on invocations in one batch.
const DEFAULT_MAX_INVOCATIONS: usize = 256;

/// The default cap on the argument bytes of one batch.
const DEFAULT_MAX_ARGUMENT_BYTES: usize = 64 * 1024;

/// Retention limits for one batch, including repeated calls to the same id.
/// These limit collected data, not what the script's engine may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommandLimits {
    pub max_invocations: usize,
    pub max_argument_bytes: usize,
}

impl Default for CommandLimits {
    fn default() -> Self {
        Self {
            max_invocations: DEFAULT_MAX_INVOCATIONS,
            max_argument_bytes: DEFAULT_MAX_ARGUMENT_BYTES,
        }
    }
}

/// The requests one script run has made so far, in call order.
#[derive(Clone, Debug)]
pub struct Batch {
    offered: CommandSet,
    schema: String,
    limits: CommandLimits,
    invocations: Vec<Invocation>,
    argument_bytes: usize,
    refusal: Option<String>,
}

impl Batch {
    /// An empty batch over the commands `offered`, whose arguments follow the
    /// host's `schema`.
    pub fn new(offered: CommandSet, schema: impl Into<String>, limits: CommandLimits) -> Self {
        Self {
            offered,
            schema: schema.into(),
            limits,
            invocations: Vec::new(),
            argument_bytes: 0,
            refusal: None,
        }
    }

    /// Admit a request for command `id` with `arguments`.
    ///
    /// # Errors
    ///
    /// The reason, when the command is not offered or is disabled, a limit
    /// would be passed, or the batch is already void. The first refusal voids
    /// the batch and is the reason every later call repeats.
    pub fn collect(&mut self, id: &str, arguments: &[u8]) -> Result<(), String> {
        if let Some(reason) = &self.refusal {
            return Err(reason.clone());
        }
        match self.admit(id, arguments) {
            Ok(invocation) => {
                self.argument_bytes = self.argument_bytes.saturating_add(arguments.len());
                self.invocations.push(invocation);
                Ok(())
            },
            Err(reason) => Err(self.refuse(reason)),
        }
    }

    fn admit(&self, id: &str, arguments: &[u8]) -> Result<Invocation, String> {
        let declaration = self
            .offered
            .get(id)
            .ok_or_else(|| format!("command `{id}` is not offered"))?;
        if let Some(reason) = &declaration.disabled_reason {
            return Err(format!("command `{id}` is disabled: {reason}"));
        }
        if self.invocations.len() >= self.limits.max_invocations {
            return Err("command batch exceeds its invocation limit".into());
        }
        let remaining = self
            .limits
            .max_argument_bytes
            .saturating_sub(self.argument_bytes);
        if arguments.len() > remaining {
            return Err("command batch exceeds its argument byte limit".into());
        }
        Ok(declaration.invocation(self.schema.as_str(), arguments.to_vec()))
    }

    /// Void the batch for a reason of the caller's, such as a request that
    /// was not well formed. The first reason stands; it is returned.
    pub fn refuse(&mut self, reason: String) -> String {
        self.refusal.get_or_insert(reason).clone()
    }

    /// Whether a request has been refused.
    pub fn is_void(&self) -> bool {
        self.refusal.is_some()
    }

    /// The collected requests, once the script has finished.
    ///
    /// # Errors
    ///
    /// The first refusal, if there was one: a void batch yields nothing.
    pub fn finish(self) -> Result<Vec<Invocation>, String> {
        match self.refusal {
            Some(reason) => Err(reason),
            None => Ok(self.invocations),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EffectClass;
    use crate::menu::Command;

    const SCHEMA: &str = "test:v1";

    fn offered() -> CommandSet {
        let mut offered = CommandSet::new();
        for (id, label) in [("view:fit", "Fit"), ("view:locked", "Locked")] {
            offered.register(Command::new(id, label, "canvas", EffectClass::Curation));
        }
        offered.set_disabled("view:locked", Some("not now".into()));
        offered
    }

    fn batch(limits: CommandLimits) -> Batch {
        Batch::new(offered(), SCHEMA, limits)
    }

    #[test]
    fn requests_are_collected_in_call_order() {
        let mut batch = batch(CommandLimits::default());
        batch.collect("view:fit", b"a").unwrap();
        batch.collect("view:fit", b"").unwrap();
        let invocations = batch.finish().unwrap();
        assert_eq!(invocations.len(), 2);
        assert_eq!(invocations[0].arguments, b"a");
        assert_eq!(invocations[0].schema_id, SCHEMA);
        assert_eq!(invocations[0].effect, EffectClass::Curation);
        assert!(invocations[1].arguments.is_empty());
    }

    #[test]
    fn an_unoffered_or_disabled_command_voids_the_batch() {
        for id in ["view:other", "view:locked"] {
            let mut batch = batch(CommandLimits::default());
            batch.collect("view:fit", b"").unwrap();
            let reason = batch.collect(id, b"").unwrap_err();
            assert!(reason.contains(id), "{reason}");
            // A valid request after the refusal repeats the first reason.
            assert_eq!(batch.collect("view:fit", b"").unwrap_err(), reason);
            assert!(batch.is_void());
            assert_eq!(batch.finish().unwrap_err(), reason);
        }
    }

    #[test]
    fn the_limits_hold_at_the_edge_and_one_past_it() {
        let limits = CommandLimits {
            max_invocations: 2,
            max_argument_bytes: 4,
        };
        // At the limits: two requests, four bytes.
        let mut at = batch(limits);
        at.collect("view:fit", b"ab").unwrap();
        at.collect("view:fit", b"cd").unwrap();
        assert_eq!(at.finish().unwrap().len(), 2);

        let mut too_many = batch(limits);
        too_many.collect("view:fit", b"").unwrap();
        too_many.collect("view:fit", b"").unwrap();
        assert!(
            too_many
                .collect("view:fit", b"")
                .unwrap_err()
                .contains("invocation limit")
        );

        let mut too_long = batch(limits);
        too_long.collect("view:fit", b"abc").unwrap();
        assert!(
            too_long
                .collect("view:fit", b"de")
                .unwrap_err()
                .contains("byte limit")
        );
        assert!(too_long.finish().is_err());
    }

    #[test]
    fn a_callers_own_refusal_voids_the_batch_and_the_first_reason_stands() {
        let mut batch = batch(CommandLimits::default());
        batch.collect("view:fit", b"").unwrap();
        assert_eq!(batch.refuse("first".into()), "first");
        assert_eq!(batch.refuse("second".into()), "first");
        assert_eq!(batch.finish().unwrap_err(), "first");
    }
}
