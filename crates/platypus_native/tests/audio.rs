// Copyright 2026 The Platypus Authors
// SPDX-License-Identifier: Apache-2.0

//! Audio from QuickJS: the commands the host receives are the ones compiled code produces. IDs are
//! in TESTS.md (M9-API-*).
mod common;
use common::{js, throws, with_world};
use platypus_engine_api::audio::{AudioCommand, take_commands};

// M9-API-132
//
// Sounds, playbacks, buses and emitters: the host sees Load / Play / Set / Stop / Bus / Emitter
// commands, `onEnded` runs when the host reports the playback finished.
#[test]
fn sounds_playbacks_buses_and_emitters_produce_host_commands() {
    with_world(|ctx| {
        let _ = take_commands();
        let out: String = js(
            ctx,
            r#"
            globalThis.ended = 0;
            const s = audio.load('sfx/hit.ogg', false);
            const bus = audio.createBus(0.5);
            const em = audio.createEmitter(new Vector3(1, 2, 3), 1, 20, bus);
            const p = s.play(0.8, 1.5, -0.25, true, 0.1, 0, bus, null);
            p.setVolume(0.2, 0.5);
            p.onEnded(() => { ended++; });
            em.setPosition(new Vector3(4, 5, 6), 0.25);
            audio.setListener(new Vector3(0, 1, 0), new Quaternion(0, 0, 0, 1));
            audio.setMasterVolume(0.9);
            bus.pause();
            p.pause(0.1);
            p.stop();
            JSON.stringify([s.path, p.sound !== null, String(s), String(p), String(bus), String(em),
                audio.soundCount, audio.masterVolume, p.paused])
            "#,
        );
        assert_eq!(
            out,
            r#"["sfx/hit.ogg",true,"[object Sound]","[object Playback]","[object Bus]","[object Emitter]",1,0.9,true]"#
        );
        let cmds = take_commands();
        assert!(
            matches!(&cmds[0], AudioCommand::Load { path, stream: false, .. } if path == "sfx/hit.ogg")
        );
        assert!(
            cmds.iter()
                .any(|c| matches!(c, AudioCommand::Bus { volume, .. } if *volume == 0.5))
        );
        assert!(cmds.iter().any(
            |c| matches!(c, AudioCommand::Emitter { position, distances, .. }
            if *position == [1.0, 2.0, 3.0] && *distances == [1.0, 20.0])
        ));
        let play = cmds
            .iter()
            .find_map(|c| match c {
                AudioCommand::Play { options, track, .. } => Some((*options, *track)),
                _ => None,
            })
            .expect("a Play command");
        assert!(play.1.is_some(), "on the bus");
        assert_eq!(play.0.volume, 0.8);
        assert_eq!(play.0.rate, 1.5);
        assert_eq!(play.0.pan, -0.25);
        assert!(play.0.looped);
        assert!(cmds.iter().any(
            |c| matches!(c, AudioCommand::Set { property: "volume", value, fade, .. }
            if *value == 0.2 && *fade == 0.5)
        ));
        assert!(cmds.iter().any(
            |c| matches!(c, AudioCommand::EmitterPosition { position, glide, .. }
            if *position == [4.0, 5.0, 6.0] && *glide == 0.25)
        ));
        assert!(cmds.iter().any(
            |c| matches!(c, AudioCommand::Listener { position, .. } if *position == [0.0, 1.0, 0.0])
        ));
        assert!(cmds.iter().any(|c| matches!(c, AudioCommand::Stop(..))));
        // The host reports the playback ended: the JS callback runs once.
        platypus_engine_api::audio::ended(&[platypus_engine_api::audio::PlaybackRef(1)]);
        let ended: f64 = js(ctx, "ended");
        assert_eq!(ended, 1.0);
        assert!(throws(ctx, "audio.load(5, false)").starts_with("TypeError"));
        assert!(throws(ctx, "audio.load('a.ogg', false).play(1, 1, 0, false, 0, 0, audio.createEmitter(new Vector3(0,0,0)), null)")
            .starts_with("TypeError"));
    });
}

// M9-API-133
//
// AudioSource and AudioListener components, a one-shot and the left-out optional parameters.
#[test]
fn audio_source_and_listener_components() {
    with_world(|ctx| {
        let _ = take_commands();
        let out: String = js(
            ctx,
            r#"
            const go = GameObject.create();
            const src = AudioSource.attach(go);
            src.clipPath = 'music/loop.ogg';
            src.volume = 0.4;
            src.loop = true;
            src.play();
            const listener = AudioListener.attach(GameObject.create());
            AudioListener.setVolume(0.7);
            const clip = audio.load('a.ogg', true);
            AudioSource.playClipAtPoint(clip, new Vector3(1, 0, 0));
            src.playOneShot(clip, 0.5);
            JSON.stringify([src.clipPath, src.volume, src.loop, src.isPlaying, src.mute, AudioListener.getVolume(),
                AudioSource.findOn(go) !== null, String(src), String(listener)])
            "#,
        );
        assert_eq!(
            out,
            r#"["music/loop.ogg",0.4,true,true,false,0.7,true,"[object AudioSource]","[object AudioListener]"]"#
        );
        let cmds = take_commands();
        assert!(
            cmds.iter()
                .filter(|c| matches!(c, AudioCommand::Play { .. }))
                .count()
                >= 3
        );
        assert!(throws(ctx, "src.volume = 'loud'").starts_with("TypeError"));
    });
}
