// SPDX-License-Identifier: Apache-2.0

// M1-SCR-01..08: the shared conformance suite, run against the QuickJS backend.
iron_script::runtime_conformance_tests!(iron_script::QuickJsRuntime::new());
