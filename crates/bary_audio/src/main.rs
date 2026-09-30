use hound;
use kira::sound::PlaybackState;
use kira::sound::static_sound::StaticSoundData;
use kira::track::TrackBuilder;
use kira::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::PI;

#[derive(Debug, Serialize, Deserialize)]
enum Note {
    B2,
    C3,
    D3,
    E3,
    F3,
    G3,
    A3,
    B3,
    C4,
    D4,
    E4,
    F4,
    G4,
    A4,
    B4,
    C5,
}

impl Note {
    pub fn to_frequency(&self) -> f32 {
        match self {
            Self::B2 => 123.5,
            Self::C3 => 130.8,
            Self::D3 => 146.8,
            Self::E3 => 164.8,
            Self::F3 => 174.6,
            Self::G3 => 196.0,
            Self::A3 => 220.0,
            Self::B3 => 123.5 * 2.0,
            Self::C4 => Self::C3.to_frequency() * 2.0,
            Self::D4 => Self::D3.to_frequency() * 2.0,
            Self::E4 => Self::E3.to_frequency() * 2.0,
            Self::F4 => Self::F3.to_frequency() * 2.0,
            Self::G4 => Self::G3.to_frequency() * 2.0,
            Self::A4 => Self::A3.to_frequency() * 2.0,
            Self::B4 => Self::B3.to_frequency() * 2.0,
            Self::C5 => Self::C4.to_frequency() * 2.0,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct SampleDescriptor {
    start_time: f32,
    duration: f32,
    amplitude: f32,
    note: Note,
}

#[derive(Debug, Serialize, Deserialize)]
struct SongDescriptor {
    samples: Vec<SampleDescriptor>,
}

impl SongDescriptor {
    fn from_file(path: &str) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        serde_yaml::from_str(&content).ok()
    }
}

fn play_sound(path: &str) {
    let mut manager = AudioManager::<DefaultBackend>::new(AudioManagerSettings::default()).unwrap();
    let builder = TrackBuilder::new();
    let mut track = manager.add_sub_track(builder).unwrap();

    let sound = StaticSoundData::from_file(path).unwrap();
    let x = track.play(sound).unwrap();

    while x.state() == PlaybackState::Playing {}

    println!("Done.");
}

fn sample_sin_wave(t: f32, freq: f32, amplitude: f32) -> f32 {
    let freq = freq * 2.0 * PI;
    (t * freq).sin() * amplitude
}

const SAMPLE_RATE: u32 = 44100;

struct Song {
    samples: Vec<Sample>,
}

impl Song {
    fn get(&self, index: i64) -> f32 {
        let mut val = 0.0;
        for sample in &self.samples {
            val += sample.get(index);
        }
        val
    }

    fn add_sample(&mut self, sample: Sample) {
        println!(
            "{} {:0.2} {:0.2}",
            sample.offset,
            sample.start_time(),
            sample.end_time()
        );
        self.samples.push(sample);
    }

    fn end_time(&self) -> f32 {
        let mut end: f32 = 0.0;
        for sample in &self.samples {
            end = end.max(sample.end_time());
        }
        end
    }
}

struct Sample {
    offset: i64,
    samples: Vec<f32>,
}

impl Sample {
    fn from_note(note: Note, start_time: f32, duration: f32, amplitude: f32) -> Self {
        let freq = note.to_frequency();
        let end_time = start_time + duration;
        let mut samples = Vec::new();
        let n_samples = (duration * SAMPLE_RATE as f32) as i64;

        let ramp_time = 0.03;
        let n_ramp_samples = (ramp_time * SAMPLE_RATE as f32) as i64;

        let resonances = [1.0, 0.4, 0.2, 0.1, 0.05, 0.05, 0.05, 0.05, 0.05];

        let sample_func = |t: f32, amplitude: f32| {
            let mut s = 0.0;
            for (i, r) in resonances.iter().enumerate() {
                let dt = t - start_time;
                dbg!(dt);
                s += sample_sin_wave(t, freq * (i + 1) as f32, amplitude * *r);
            }
            s
        };

        for n in 0..n_ramp_samples {
            let s = n as f32 / n_ramp_samples as f32;
            let t = start_time - (1.0 - s) * ramp_time;
            let sample = sample_func(t, amplitude * s);
            samples.push(sample);
        }

        for n in 0..n_samples {
            let t = start_time + n as f32 / SAMPLE_RATE as f32;
            let sample = sample_func(t, amplitude);
            samples.push(sample);
        }

        for n in 0..n_ramp_samples {
            let s = n as f32 / n_ramp_samples as f32;
            let t = end_time + s * ramp_time;
            let sample = sample_func(t, amplitude * (1.0 - s));
            samples.push(sample);
        }

        let offset = (start_time * SAMPLE_RATE as f32) as i64;

        Self {
            offset: offset - n_ramp_samples,
            samples,
        }
    }

    fn start_time(&self) -> f32 {
        self.offset as f32 / SAMPLE_RATE as f32
    }

    fn end_time(&self) -> f32 {
        self.start_time() + self.duration()
    }

    fn duration(&self) -> f32 {
        self.samples.len() as f32 / SAMPLE_RATE as f32
    }

    fn get(&self, index: i64) -> f32 {
        let idx = index - self.offset;
        if idx < 0 {
            return 0.0;
        }
        self.samples.get(idx as usize).cloned().unwrap_or(0.0)
    }
}

fn main() {
    let infile = std::env::args().nth(1).unwrap();
    let outfile = std::env::args().nth(2).unwrap();

    let song_desc = SongDescriptor::from_file(&infile).unwrap();

    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: SAMPLE_RATE,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };

    let mut song = Song {
        samples: Vec::new(),
    };

    for sample in song_desc.samples {
        let sample = Sample::from_note(
            sample.note,
            sample.start_time,
            sample.duration,
            sample.amplitude * 0.02,
        );
        song.add_sample(sample);
    }

    println!("{:0.2} seconds", song.end_time());

    let mut writer = hound::WavWriter::create(&outfile, spec).unwrap();

    let n_samples = (SAMPLE_RATE as f32 * (song.end_time() + 1.0)) as i64;

    for n in 0..n_samples {
        let sample = song.get(n as i64);

        writer.write_sample(sample).unwrap();
        writer.write_sample(sample).unwrap();
    }

    writer.finalize().unwrap();

    play_sound(&outfile);
}
