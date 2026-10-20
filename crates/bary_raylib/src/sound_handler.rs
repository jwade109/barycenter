use crate::SoundEffects;
use log::error;
use raylib::audio::{RaylibAudio, RaylibAudioInitError, Sound};
use std::{
    sync::mpsc::{Receiver, Sender},
    time::Duration,
};

pub struct SoundHandler {
    sender: Sender<SoundEffects>,
}

impl SoundHandler {
    pub fn new() -> Self {
        let (sender, receiver) = std::sync::mpsc::channel();

        std::thread::spawn(|| sound_thread(receiver));

        Self { sender }
    }

    pub fn play_sounds(&self, sounds: SoundEffects) {
        _ = self.sender.send(sounds)
    }
}

fn sound_thread(sounds: Receiver<SoundEffects>) -> Result<(), RaylibAudioInitError> {
    let audio = raylib::audio::RaylibAudio::init_audio_device()?;
    let mut active_sounds = Vec::new();

    loop {
        for sounds in sounds.try_iter() {
            handle_sounds(sounds, &audio, &mut active_sounds);
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

fn handle_sounds<'a>(
    sounds: SoundEffects,
    audio: &'a RaylibAudio,
    active_sounds: &mut Vec<Sound<'a>>,
) {
    for sound in sounds {
        let path = sound.to_path();
        let sound = match audio.new_sound(path) {
            Ok(s) => s,
            Err(e) => {
                error!("Failed to load sound: {}", e);
                continue;
            }
        };

        sound.set_volume(0.4);
        sound.play();

        active_sounds.push(sound);
    }

    active_sounds.retain(|s| s.is_playing());
}
