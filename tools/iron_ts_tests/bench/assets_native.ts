// Assets.exists / list / packOf from compiled code. The Rust test (tests/all/assets_native.rs)
// fills the index the host would, and checks what the program gets.

export function files(): string {
  const wavs = Assets.list("assets/sfx/*.wav").split("\n");
  const boss = Assets.packOf("assets/sfx/b.wav") === "scene-boss" ? "boss" : Assets.packOf("assets/sfx/b.wav");
  const none = Assets.packOf("assets/none.wav") === "" ? "none" : "?";
  const empty = Assets.list("nothing/**") === "" ? "empty" : "?";
  return `${wavs.length}|${wavs.join(",")}|${Assets.exists("assets/sfx/a.wav")}|${Assets.exists("assets/sfx/none.wav")}|${Assets.exists("data:,x")}|${boss}|${none}|${empty}`;
}
