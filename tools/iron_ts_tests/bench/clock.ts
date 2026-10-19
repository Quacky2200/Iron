// TimeSince and TimeUntil on the game clock, and Time's real-time reads, compiled against the
// native API and run in bun against js/engine/src/time.ts. The clock has run three frames of
// 0.25 s (0.75 s) before the program starts.

function f(x: number): string {
  return x.toFixed(4);
}

export function since(): string {
  const a = TimeSince.now();
  const b = TimeSince.ago(2);
  return `${f(a.seconds)},${f(b.seconds)},${a.over(0.5)},${b.over(1.5)},${b.over(2)}`;
}

export function until(): string {
  const a = TimeUntil.in(2);
  const zero = TimeUntil.in(0);
  const past = TimeUntil.in(-1);
  return `${f(a.seconds)},${a.done},${f(a.fraction)},${zero.done},${f(zero.fraction)},${f(past.seconds)},${past.done},${f(past.fraction)}`;
}

export function afterTime(): string {
  const a = TimeUntil.in(1);
  const s = TimeSince.now();
  Time.timeScale = 2;
  return `${f(a.seconds)},${f(s.seconds)},${Time.frameCount},${f(Time.time)},${f(Time.deltaTime)},${Time.timeScale}`;
}
