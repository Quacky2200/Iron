// The engine ticks the clock three times before the game's code runs.
const g = globalThis as any;
g.__iron_tick(0.25);
g.__iron_tick(0.25);
g.__iron_tick(0.25);
