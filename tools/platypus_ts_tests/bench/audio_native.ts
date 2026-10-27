// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

// The native audio API from compiled code: sounds, playbacks, buses, emitters, and the
// AudioSource / AudioListener components. The Rust test (tests/engine_api/audio_native.rs) reads the
// commands the host would run on the mixer.

let ended = "";

export function playbacks(): string {
  const hit = audio.load("sfx/hit.wav");
  const music = audio.load("music/theme.ogg", { stream: true });
  const bus = audio.createBus({ volume: 0.5 });
  const sub = audio.createBus({ volume: 0.25, parent: bus });
  const p = hit.play({ volume: 0.8, rate: 1.5, loop: true, bus: sub });
  p.setVolume(0.4, 0.2);
  p.setPan(-1);
  p.pause();
  p.resume(0.1);
  p.seek(1.5);
  p.onEnded(() => {
    ended += "p;";
  });
  p.stop(0.3);
  const q = music.play();
  bus.setVolume(0.9, 1);
  bus.pause();
  sub.remove();
  audio.setMasterVolume(0.7);
  return `${hit.path}|${music.src}|${p.playing}|${q.playing}|${q.sound.path}|${audio.masterVolume.toFixed(1)}`;
}

export function playByPath(): string {
  const a = audio.play("ui/click.wav");
  const b = audio.play("ui/click.wav", { volume: 0.5 });
  return `${audio.soundCount}|${a.sound.path}|${b.sound.path}`;
}

export function emitterAndListener(): string {
  const e = audio.createEmitter(new Vector3(1, 2, 3), { minDistance: 2, maxDistance: 20 });
  e.setPosition(new Vector3(4, 5, 6), 0.5);
  e.setVolume(0.5);
  audio.setListener(new Vector3(0, 1, 0), new Quaternion(0, 0, 0, 1));
  e.remove();
  return "ok";
}

export function endedLog(): string {
  return ended;
}

export function source(): number {
  const go = new GameObject("Speaker", { position: new Vector3(2, 0, 0) });
  const hit = audio.load("sfx/hit.wav");
  const src = go.addComponent(AudioSource, { clip: hit, spatialBlend: 1, minDistance: 3, volume: 0.5, loop: true });
  src.play();
  src.pitch = 2;
  src.mute = true;
  src.playOneShot(hit, 0.5);
  return src.isPlaying ? 1 : 0;
}

export function sourceFlat(): string {
  const go = new GameObject("Flat");
  const src = go.addComponent(AudioSource, { playOnAwake: false });
  src.clip = hitSound();
  src.panStereo = -0.5;
  src.play();
  src.pause();
  const paused = src.isPlaying;
  src.unPause();
  const resumed = src.isPlaying;
  src.stop();
  return `${paused}|${resumed}|${src.isPlaying}|${go.getComponent(AudioSource) === src}`;
}

function hitSound(): Sound {
  return audio.load("sfx/hit.wav");
}

export function listenerFollows(): number {
  const go = new GameObject("Ears", { position: new Vector3(0, 2, 0) });
  go.addComponent(AudioListener);
  return AudioListener.getVolume();
}

export function clipAtPoint(): number {
  const hit = audio.load("sfx/hit.wav");
  const p = AudioSource.playClipAtPoint(hit, new Vector3(1, 1, 1), 0.5);
  return p.playing ? 1 : 0;
}
