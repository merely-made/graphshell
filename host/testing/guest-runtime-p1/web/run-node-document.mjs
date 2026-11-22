// Copyright 2026 Mark Alan Boykin
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.
// SPDX-License-Identifier: MPL-2.0

// Node runner for the document guest probe: prints one JSON object with a
// result per case and exits nonzero unless every case and control passes.

import { runDocument, toJson } from './run-document.js';

const result = await runDocument();
console.log(toJson(result));
process.exit(result.passed ? 0 : 1);
