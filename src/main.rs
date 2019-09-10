//! Play a sine wave for several seconds.
//!
//! A rusty adaptation of the official PortAudio C "paex_sine.c" example by Phil Burk and Ross
//! Bencina.

extern crate portaudio;
extern crate rsynth_data_import as rdimport;

use portaudio as pa;
use std::f64::consts::PI;

const CHANNELS: i32 = 2;
const NUM_SECONDS: i32 = 5;
const SAMPLE_RATE: f64 = 44_100.0;
const FRAMES_PER_BUFFER: u32 = 64;

fn main() {
    match run() {
        Ok(_) => {}
        e => {
            eprintln!("Example failed with the following: {:?}", e);
        }
    }
}

trait MonoSample {
    fn value_at(&self, idx: f32) -> f32;
}

struct Wavetable {
    data: Vec<f32>,
}

impl MonoSample for rdimport::WaveData {
    // fn value_at(&self, idx: f32) -> f32 {
    //     let data_pos = (idx * self.len() as f32) as usize % self.len();
    //     self[data_pos].1 as f32
    // }

    fn value_at(&self, idx: f32) -> f32 {
        match self.binary_search_by_key(&idx, |&(time, amp)| time) {
            Ok(target_idx) => self.get(target_idx).unwrap().1 as _,
            Err(right_idx) => lerp_with_index(self, idx, right_idx),
        }
    }
}

fn lerp_with_index(table: &rdimport::WaveData, target_idx: f32, right_idx: usize) -> f32 {
    let left_frame = if right_idx == 0 {
        let left_idx = table.len() - 1;
        table
            .get(left_idx)
            .map(|(t, a)| (t + 1., a.clone()))
            .unwrap()
    } else {
        let left_idx = right_idx - 1;
        table.get(left_idx).unwrap().clone()
    };

    let right_frame = table.get(right_idx).unwrap();

    lerp_frames(target_idx, left_frame, *right_frame)
}

fn lerp_frames(
    target_idx: f32,
    left_frame: rdimport::WaveFrame,
    right_frame: rdimport::WaveFrame,
) -> f32 {
    (if left_frame.0 == right_frame.0 {
        left_frame.1
    } else {
        let amp_range = right_frame.1 - right_frame.1;
        let time_range = right_frame.0 - left_frame.0;
        left_frame.1 + (target_idx as f64 - left_frame.0) * (amp_range / time_range)
    }) as f32
}

impl MonoSample for Wavetable {
    fn value_at(&self, idx: f32) -> f32 {
        let data_pos = (idx * self.data.len() as f32) as usize % self.data.len();
        self.data[data_pos as usize]
    }
}

impl Wavetable {
    fn new(table_size: usize) -> Wavetable {
        let mut sine = Vec::with_capacity(table_size);
        for i in 0..table_size {
            sine.push((i as f64 / table_size as f64 * PI * 2.0).sin() as f32);
        }
        Wavetable { data: sine }
    }
}

fn run() -> Result<(), pa::Error> {
    println!(
        "PortAudio Test: output sine wave. SR = {}, BufSize = {}",
        SAMPLE_RATE, FRAMES_PER_BUFFER
    );

    // Initialise sinusoidal wavetable.
    let wavetable = if true {
        Box::new(rdimport::load_csv_data("../rsynth-data-import/asset/data.csv".as_ref()).unwrap())
            as Box<MonoSample>
    } else {
        Box::new(Wavetable::new(200)) as Box<MonoSample>
    };

    let pa = pa::PortAudio::new()?;

    let mut settings =
        pa.default_output_stream_settings(CHANNELS, SAMPLE_RATE, FRAMES_PER_BUFFER)?;
    // we won't output out of range samples so don't bother clipping them.
    settings.flags = pa::stream_flags::CLIP_OFF;

    let pitch = 220.;
    // TODO: math this correctly ((???))
    let samples_per_period = SAMPLE_RATE * pitch;
    let mut play_position = 0.;
    let mut idx = 0;

    // This routine will be called by the PortAudio engine when audio is needed. It may called at
    // interrupt level on some machines so don't do anything that could mess up the system like
    // dynamic resource allocation or IO.
    let callback = move |pa::OutputStreamCallbackArgs { buffer, frames, .. }| {
        let mut idx = 0;
        for frame in 0..frames {
            // TODO: math this more correcterer as well
            // play_position += (1. / frames as f32) * samples_per_period as f32;
            play_position += 0.000625;
            play_position %= 1.;

            let sample = wavetable.value_at(play_position);

            buffer[idx] = sample;
            buffer[idx + 1] = sample;
            idx += 2;
        }
        pa::Continue
    };

    let mut stream = pa.open_non_blocking_stream(settings, callback)?;

    stream.start()?;

    println!("Play for {} seconds.", NUM_SECONDS);
    pa.sleep(NUM_SECONDS * 1_000);

    stream.stop()?;
    stream.close()?;

    println!("Test finished.");

    Ok(())
}
