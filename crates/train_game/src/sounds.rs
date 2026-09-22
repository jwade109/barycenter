use crate::event_bus::*;
use bary_core::prelude::*;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::*;
use kira::*;
use log::info;
use std::collections::BTreeMap;
use std::time::Duration;

#[derive(Debug)]
pub struct Sound {
    pub name: String,
    pub duration: Duration,
    pub handle: StaticSoundHandle,
}

impl std::fmt::Display for Sound {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: {:0.4}/{:0.4} {:?}",
            self.name,
            self.handle.position(),
            self.duration.as_secs_f64(),
            self.handle.state(),
        )
    }
}

pub struct SoundManager {
    _manager: AudioManager,
    track: TrackHandle,
    spawner: EntitySpawner,
    sounds: Components<Sound>,
    loaded_sounds: BTreeMap<SoundKind, StaticSoundData>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SoundKind {
    ButtonUp,
    Crossword,
    HouseOfLeaves,
}

impl SoundKind {
    fn path(&self) -> &'static str {
        match self {
            SoundKind::ButtonUp => "assets/sfx/button-up.ogg",
            SoundKind::Crossword => "assets/sfx/nyt-crossword.ogg",
            SoundKind::HouseOfLeaves => "assets/sfx/house_of_leaves.mp3",
        }
    }
}

impl SoundManager {
    pub fn new() -> Self {
        let mut _manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
        let builder = TrackBuilder::new();
        let track = _manager.add_sub_track(builder).unwrap();

        let mut loaded_sounds = BTreeMap::new();

        for kind in [
            SoundKind::ButtonUp,
            SoundKind::Crossword,
            SoundKind::HouseOfLeaves,
        ] {
            let path = kind.path();
            let sound = StaticSoundData::from_file(path).unwrap();
            loaded_sounds.insert(kind, sound);
        }

        Self {
            _manager,
            track,
            spawner: EntitySpawner::default(),
            sounds: Components::default(),
            loaded_sounds,
        }
    }

    pub fn update(&mut self) {
        self.sounds.retain(|_id, sound| {
            if sound.handle.state() == PlaybackState::Stopped {
                false
            } else {
                true
            }
        });
    }

    fn spawn_sound(&mut self, kind: SoundKind) -> Result<(), Box<dyn std::error::Error>> {
        info!("playing sound {kind:?}");
        let sound = self.loaded_sounds.get(&kind).unwrap();
        let mut handle = self.track.play(sound.clone())?;
        let pbr = 1.0; // bary_core::prelude::rand(0.5, 2.0) as f64 * 0.1;
        handle.set_playback_rate(pbr, kira::Tween::default());
        let id = self.spawner.spawn();
        let sound = Sound {
            name: format!("{:?}", kind),
            duration: sound.duration(),
            handle,
        };
        self.sounds.spawn(id, sound);
        Ok(())
    }

    fn remove_sound(&mut self, id: Ent) -> Option<()> {
        let mut sound = self.sounds.despawn(id).ok()?;
        sound.handle.stop(Tween {
            duration: Duration::from_millis(1000),
            ..Default::default()
        });
        Some(())
    }

    pub fn handle_events(&mut self, events: &EventBus<TrainEvent>) {
        for event in events.iter() {
            match event {
                TrainEvent::KillSound(id) => {
                    self.remove_sound(*id);
                }
                TrainEvent::Sound(s) => _ = self.spawn_sound(*s),
                _ => (),
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (&Ent, &Sound)> {
        self.sounds.iter()
    }
}
