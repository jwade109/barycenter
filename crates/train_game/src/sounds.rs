use crate::event_bus::*;
use bary_core::prelude::*;
use kira::sound::PlaybackState;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use kira::track::*;
use kira::*;
use log::info;
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
}

#[derive(Debug, Clone, Copy)]
pub enum SoundKind {
    ButtonUp,
    Crossword,
    HouseOfLeaves,
}

impl SoundManager {
    pub fn new() -> Self {
        let mut _manager =
            AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
        let builder = TrackBuilder::new();
        let track = _manager.add_sub_track(builder).unwrap();

        Self {
            _manager,
            track,
            spawner: EntitySpawner::default(),
            sounds: Components::default(),
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

    fn play_sound(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        info!("playing sound {path}");
        let sound = StaticSoundData::from_file(path)?;
        let mut handle = self.track.play(sound.clone())?;
        let pbr = 1.0; // bary_core::prelude::rand(0.5, 2.0) as f64 * 0.1;
        handle.set_playback_rate(pbr, kira::Tween::default());
        let id = self.spawner.spawn();
        let sound = Sound {
            name: path.into(),
            duration: sound.duration(),
            handle,
        };
        self.sounds.spawn(id, sound);
        Ok(())
    }

    fn spawn_sound(&mut self, s: SoundKind) {
        let path = match s {
            SoundKind::ButtonUp => "assets/sfx/button-up.ogg",
            SoundKind::Crossword => "assets/sfx/button-up.ogg",
            SoundKind::HouseOfLeaves => "assets/sfx/house_of_leaves.mp3",
        };
        self.play_sound(path);
    }

    fn remove_sound(&mut self, id: Ent) -> Option<()> {
        let mut sound = self.sounds.despawn(id).ok()?;
        sound.handle.stop(Tween::default());
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
