// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// M1-SCR-01..08: the shared conformance suite, run against the QuickJS backend.
platypus_script::runtime_conformance_tests!(platypus_script::QuickJsRuntime::new());
