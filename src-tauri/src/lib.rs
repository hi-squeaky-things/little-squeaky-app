use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, Stream, StreamConfig};
use include_dir::{include_dir, File};
use little_weirdo::synth::data::patches::{BoxedPatch, BoxedPatches};
use little_weirdo::synth::data::waveforms::{BoxedWaveform, BoxedWaveforms};
use little_weirdo::synth::patch::{Patch, SynthMode};
use little_weirdo::synth::Synth;
use serde::Serialize;
use std::collections::VecDeque;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

static SOUNDBANK: include_dir::Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/src/soundbank");

enum AudioEvent {
    NoteOn { note: u8, velocity: u8 },
    NoteOff { note: u8 },
    SelectWaveform { waveform: u8 },
    SelectPatch { patch: u8 },
}

struct AudioEngine {
    events: Sender<AudioEvent>,
    output: Arc<Mutex<VecDeque<i16>>>,
    _stream: Stream,
}

impl AudioEngine {
    fn new() -> Result<Self, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default audio output device is available".to_string())?;
        let supported_config = device
            .default_output_config()
            .map_err(|error| format!("Could not read audio output configuration: {error}"))?;
        let sample_rate = supported_config.sample_rate().min(u16::MAX as u32) as u16;
        let config: StreamConfig = supported_config.clone().into();
        let (events, receiver) = mpsc::channel();
        let output = Arc::new(Mutex::new(VecDeque::with_capacity(2048)));
        let stream = match supported_config.sample_format() {
            SampleFormat::F32 => build_stream::<f32>(
                &device,
                config,
                create_synth(sample_rate),
                receiver,
                Arc::clone(&output),
            ),
            SampleFormat::I16 => build_stream::<i16>(
                &device,
                config,
                create_synth(sample_rate),
                receiver,
                Arc::clone(&output),
            ),
            SampleFormat::U16 => build_stream::<u16>(
                &device,
                config,
                create_synth(sample_rate),
                receiver,
                Arc::clone(&output),
            ),
            _format => Err(cpal::BuildStreamError::StreamConfigNotSupported),
        }
        .map_err(|error| format!("Could not create audio output stream: {error}"))?;
        stream
            .play()
            .map_err(|error| format!("Could not start audio output stream: {error}"))?;

        Ok(Self {
            events,
            output,
            _stream: stream,
        })
    }
}

fn create_synth(sample_rate: u16) -> Synth {
    let mut waveforms = BoxedWaveforms::new();
    for waveform in WAVEFORMS {
        waveforms.add(BoxedWaveform::new(waveform));
    }

    let patches = create_patches();

    Synth::new(sample_rate, 0, Arc::new(patches), Arc::new(waveforms))
}

fn create_patches() -> BoxedPatches {
    let mut patches = BoxedPatches::new();
    let patch_files = patch_files();
    for file in &patch_files {
        let patch: Patch = serde_json::from_slice(file.contents())
            .unwrap_or_else(|error| panic!("invalid patch {}: {error}", file.path().display()));
        patches.add(BoxedPatch::new(patch));
    }
    let template = patch_files
        .first()
        .expect("at least one JSON patch must exist in the soundbank");
    for waveform in 0..WAVEFORMS.len() as u8 {
        let mut patch: Patch = serde_json::from_slice(template.contents())
            .expect("soundbank patch template must be valid");
        patch.name = format!("waveform_{waveform}");
        for voice in &mut patch.voices {
            voice.soundbank_index = waveform;
        }
        patches.add(BoxedPatch::new(patch));
    }
    patches
}

fn patch_files() -> Vec<&'static File<'static>> {
    let mut files: Vec<_> = SOUNDBANK
        .files()
        .filter(|file| file.path().extension().is_some_and(|extension| extension == "json"))
        .collect();
    files.sort_by_key(|file| file.path());
    files
}

#[derive(Serialize)]
struct PatchInfo {
    name: String,
    mode: String,
}

fn build_stream<T>(
    device: &cpal::Device,
    config: StreamConfig,
    mut synth: Synth,
    receiver: Receiver<AudioEvent>,
    output_buffer: Arc<Mutex<VecDeque<i16>>>,
) -> Result<Stream, cpal::BuildStreamError>
where
    T: cpal::SizedSample + FromSample<i16>,
{
    let channels = config.channels as usize;
    device.build_output_stream(
        &config,
        move |data: &mut [T], _| {
            while let Ok(event) = receiver.try_recv() {
                match event {
                    AudioEvent::NoteOn { note, velocity } => synth.note_on(note, velocity),
                    AudioEvent::NoteOff { note } => synth.note_off(note),
                    AudioEvent::SelectWaveform { waveform } => {
                        synth.all_note_off();
                        synth.load_patch(patch_files().len() as u8 + waveform);
                    }
                    AudioEvent::SelectPatch { patch } => {
                        synth.all_note_off();
                        synth.load_patch(patch);
                    }
                }
            }

            for frame in data.chunks_mut(channels) {
                let output = synth.clock_and_output();
                if let Ok(mut samples) = output_buffer.lock() {
                    samples.push_back(output[0]);
                    while samples.len() > 2048 {
                        samples.pop_front();
                    }
                }
                for (channel, sample) in frame.iter_mut().enumerate() {
                    let source = output[channel.min(1)];
                    *sample = T::from_sample(source);
                }
            }
        },
        |error| eprintln!("Little Weirdo audio stream error: {error}"),
        None,
    )
}

const WAVEFORMS: [&[u8]; 56] = [
    include_bytes!("soundbank/waveforms/000_sample.raw"),
    include_bytes!("soundbank/waveforms/001_sample.raw"),
    include_bytes!("soundbank/waveforms/002_sample.raw"),
    include_bytes!("soundbank/waveforms/003_sample.raw"),
    include_bytes!("soundbank/waveforms/004_sample.raw"),
    include_bytes!("soundbank/waveforms/005_sample.raw"),
    include_bytes!("soundbank/waveforms/006_sample.raw"),
    include_bytes!("soundbank/waveforms/007_sample.raw"),
    include_bytes!("soundbank/waveforms/008_sample.raw"),
    include_bytes!("soundbank/waveforms/009_sample.raw"),
    include_bytes!("soundbank/waveforms/010_sample.raw"),
    include_bytes!("soundbank/waveforms/011_sample.raw"),
    include_bytes!("soundbank/waveforms/012_sample.raw"),
    include_bytes!("soundbank/waveforms/013_sample.raw"),
    include_bytes!("soundbank/waveforms/014_sample.raw"),
    include_bytes!("soundbank/waveforms/015_sample.raw"),
    include_bytes!("soundbank/waveforms/016_sample.raw"),
    include_bytes!("soundbank/waveforms/017_sample.raw"),
    include_bytes!("soundbank/waveforms/018_sample.raw"),
    include_bytes!("soundbank/waveforms/019_sample.raw"),
    include_bytes!("soundbank/waveforms/020_sample.raw"),
    include_bytes!("soundbank/waveforms/021_sample.raw"),
    include_bytes!("soundbank/waveforms/022_sample.raw"),
    include_bytes!("soundbank/waveforms/023_sample.raw"),
    include_bytes!("soundbank/waveforms/024_sample.raw"),
    include_bytes!("soundbank/waveforms/025_sample.raw"),
    include_bytes!("soundbank/waveforms/026_sample.raw"),
    include_bytes!("soundbank/waveforms/027_sample.raw"),
    include_bytes!("soundbank/waveforms/028_sample.raw"),
    include_bytes!("soundbank/waveforms/029_sample.raw"),
    include_bytes!("soundbank/waveforms/030_sample.raw"),
    include_bytes!("soundbank/waveforms/031_sample.raw"),
    include_bytes!("soundbank/waveforms/032_sample.raw"),
    include_bytes!("soundbank/waveforms/033_sample.raw"),
    include_bytes!("soundbank/waveforms/034_sample.raw"),
    include_bytes!("soundbank/waveforms/035_sample.raw"),
    include_bytes!("soundbank/waveforms/036_sample.raw"),
    include_bytes!("soundbank/waveforms/037_sample.raw"),
    include_bytes!("soundbank/waveforms/038_sample.raw"),
    include_bytes!("soundbank/waveforms/039_sample.raw"),
    include_bytes!("soundbank/waveforms/040_sample.raw"),
    include_bytes!("soundbank/waveforms/041_sample.raw"),
    include_bytes!("soundbank/waveforms/042_sample.raw"),
    include_bytes!("soundbank/waveforms/043_sample.raw"),
    include_bytes!("soundbank/waveforms/044_sample.raw"),
    include_bytes!("soundbank/waveforms/045_sample.raw"),
    include_bytes!("soundbank/waveforms/046_sample.raw"),
    include_bytes!("soundbank/waveforms/047_sample.raw"),
    include_bytes!("soundbank/waveforms/048_sample.raw"),
    include_bytes!("soundbank/waveforms/049_sample.raw"),
    include_bytes!("soundbank/waveforms/050_sample.raw"),
    include_bytes!("soundbank/waveforms/051_sample.raw"),
    include_bytes!("soundbank/waveforms/052_sample.raw"),
    include_bytes!("soundbank/waveforms/053_sample.raw"),
    include_bytes!("soundbank/waveforms/054_sample.raw"),
    include_bytes!("soundbank/waveforms/055_sample.raw"),
];

#[tauri::command]
fn play_note(
    note: u8,
    velocity: u8,
    engine: tauri::State<'_, Option<AudioEngine>>,
) -> Result<(), String> {
    if !(24..=108).contains(&note) {
        return Err("Note must be between MIDI 24 and 108".to_string());
    }

    engine
        .as_ref()
        .ok_or_else(|| "Audio output is unavailable".to_string())?
        .events
        .send(AudioEvent::NoteOn { note, velocity })
        .map_err(|_| "Audio engine is no longer available".to_string())
}

#[tauri::command]
fn stop_note(note: u8, engine: tauri::State<'_, Option<AudioEngine>>) -> Result<(), String> {
    if !(24..=108).contains(&note) {
        return Err("Note must be between MIDI 24 and 108".to_string());
    }

    engine
        .as_ref()
        .ok_or_else(|| "Audio output is unavailable".to_string())?
        .events
        .send(AudioEvent::NoteOff { note })
        .map_err(|_| "Audio engine is no longer available".to_string())
}

#[tauri::command]
fn select_waveform(
    waveform: u8,
    engine: tauri::State<'_, Option<AudioEngine>>,
) -> Result<(), String> {
    if waveform as usize >= WAVEFORMS.len() {
        return Err("Waveform must be between 0 and 55".to_string());
    }

    engine
        .as_ref()
        .ok_or_else(|| "Audio output is unavailable".to_string())?
        .events
        .send(AudioEvent::SelectWaveform { waveform })
        .map_err(|_| "Audio engine is no longer available".to_string())
}

#[tauri::command]
fn get_patches() -> Result<Vec<PatchInfo>, String> {
    patch_files()
        .into_iter()
        .map(|file| {
            let patch: Patch = serde_json::from_slice(file.contents())
                .map_err(|error| format!("invalid patch {}: {error}", file.path().display()))?;
            let mode = match patch.synth_config.mode {
                SynthMode::Mono => "Mono",
                SynthMode::BiPoly => "BiPoly",
                SynthMode::QuadPoly => "QuadPoly",
                SynthMode::OctoPoly => "OctoPoly",
            };
            Ok(PatchInfo {
                name: patch.name,
                mode: mode.to_string(),
            })
        })
        .collect()
}

#[tauri::command]
fn select_patch(
    patch: u8,
    engine: tauri::State<'_, Option<AudioEngine>>,
) -> Result<(), String> {
    let patch_count = patch_files().len();
    if patch as usize >= patch_count {
        return Err(format!("Patch must be between 0 and {}", patch_count - 1));
    }

    engine
        .as_ref()
        .ok_or_else(|| "Audio output is unavailable".to_string())?
        .events
        .send(AudioEvent::SelectPatch { patch })
        .map_err(|_| "Audio engine is no longer available".to_string())
}

#[tauri::command]
fn get_waveform(waveform: u8) -> Result<Vec<i16>, String> {
    let samples = WAVEFORMS
        .get(waveform as usize)
        .ok_or_else(|| "Waveform must be between 0 and 55".to_string())?;

    Ok(samples
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]))
        .collect())
}

#[tauri::command]
fn get_audio_output(engine: tauri::State<'_, Option<AudioEngine>>) -> Vec<i16> {
    engine
        .as_ref()
        .and_then(|engine| engine.output.lock().ok())
        .map(|samples| samples.iter().copied().collect())
        .unwrap_or_default()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let audio_engine = AudioEngine::new().ok();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(audio_engine)
        .invoke_handler(tauri::generate_handler![
            play_note,
            stop_note,
            select_waveform,
            get_patches,
            select_patch,
            get_waveform,
            get_audio_output
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
