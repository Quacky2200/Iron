// React Fast Refresh for dev builds (`vendor.ts`). This must run before React and the
// reconciler load: it installs the devtools hook the reconciler registers with, and the
// `$RefreshReg$` / `$RefreshSig$` globals that Bun's `--react-fast-refresh` transform
// calls in the game's code. After the game's new code has run, `performReactRefresh`
// re-renders the components whose code changed, keeping their state.
import * as RefreshRuntime from "react-refresh/runtime";

RefreshRuntime.injectIntoGlobalHook(globalThis);

const g = globalThis as unknown as {
  $RefreshReg$: (type: unknown, id: string) => void;
  $RefreshSig$: () => (type: unknown, key: string, forceReset?: boolean, getCustomHooks?: () => unknown[]) => unknown;
};
g.$RefreshReg$ = (type, id) => RefreshRuntime.register(type, id);
g.$RefreshSig$ = RefreshRuntime.createSignatureFunctionForTransform;

/** Re-renders components whose code changed (state kept where the hooks still match). */
export function performRefresh(): void {
  RefreshRuntime.performReactRefresh();
}
