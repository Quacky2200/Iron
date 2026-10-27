// SPDX-License-Identifier: Apache-2.0

// A stand-in for `property` from @platypus/engine, so bun can run bench/props.ts. The compiler never
// reads imports: it recognises the decorator by name and records the field.
export function property(_value: unknown, _context: unknown): void {}
