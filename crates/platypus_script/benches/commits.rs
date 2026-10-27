// SPDX-License-Identifier: Apache-2.0

//! M2-BEN-04: cost of sending UI commits from JS to Rust through QuickJS.
//!
//! Run with `cargo bench -p platypus_script`.

use std::hint::black_box;

use criterion::{Criterion, criterion_group, criterion_main};
use platypus_script::{QuickJsRuntime, ScriptRuntime};

fn benches(c: &mut Criterion) {
    let mut rt = QuickJsRuntime::new();
    rt.load_bundle(
        "bench.js",
        r##"
        // 1,000 ops shaped like a React commit: creates, style sets and appends.
        function makeOps(n) {
          const ops = [];
          for (let i = 0; i < n; i++) {
            switch (i % 3) {
              case 0: ops.push([0, i + 1, "box"]); break;
              case 1: ops.push([1, i, "width", 100, "backgroundColor", "#336699"]); break;
              default: ops.push([2, 0, i - 1]);
            }
          }
          return ops;
        }
        const prebuilt = makeOps(1000);
        // Only the transfer: JS array -> Rust Commit.
        function sendPrebuilt() { __platypus_commit(prebuilt); }
        // What the reconciler really does: build the ops, then send them.
        function buildAndSend() { __platypus_commit(makeOps(1000)); }
        "##,
    )
    .unwrap();

    c.bench_function("commit_transfer_1000_ops", |b| {
        b.iter(|| {
            rt.call_global("sendPrebuilt", &[]).unwrap();
            black_box(rt.drain_commits())
        })
    });

    c.bench_function("commit_build_and_transfer_1000_ops", |b| {
        b.iter(|| {
            rt.call_global("buildAndSend", &[]).unwrap();
            black_box(rt.drain_commits())
        })
    });
}

criterion_group!(commits, benches);
criterion_main!(commits);
