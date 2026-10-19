// The saved file, installed the way the engine does (and read again: the module read it at import).
import { _resetPrefsForTests } from "../../../js/engine/src/prefs.ts";

(globalThis as any).__iron_op = () => {};
(globalThis as any).__iron_info = { prefs: { "saved.n": 7.9, "saved.s": "hello" } };
_resetPrefsForTests();
