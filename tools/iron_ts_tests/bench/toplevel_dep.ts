// Imported by toplevel.ts: control-flow statements at the top of a module run too.
export const depLog: string[] = [];
export let depTotal = 0;

depLog.push("dep-start");
for (let i = 0; i < 3; i++) {
  depTotal += i + 1;
}
if (depTotal === 6) {
  depLog.push("dep-six");
} else {
  depLog.push("dep-other");
}
{
  const i = 100;
  depTotal += i;
}
