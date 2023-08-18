use std::{
    io::{self, ErrorKind},
    thread,
};

use cpal::FromSample;
use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    SizedSample,
};
use icy_engine::{AnsiMusic, EngineResult};

pub fn play_music(music: &AnsiMusic) {
    let mut i = 0;
    let mut cur_style = icy_engine::MusicStyle::Normal;

    while i < music.music_actions.len() {
        let act = &music.music_actions[i];
        i += 1;
        match act {
            icy_engine::MusicAction::SetStyle(style) => {
                cur_style = *style;
            }
            icy_engine::MusicAction::PlayNote(freq, length) => {
                let f = *freq;

                let mut duration = 250_000_u64 / u64::from(*length);

                let pause_length = match cur_style {
                    icy_engine::MusicStyle::Legato => duration / 4,
                    icy_engine::MusicStyle::Staccato => 0,
                    _ => duration / 8,
                };
                duration -= pause_length;
                {
                    let stream = stream_setup_for(move |o| {
                        o.tick();
                        o.tone(f)
                    })
                    .unwrap();
                    stream.play().unwrap();
                    std::thread::sleep(std::time::Duration::from_millis(duration));
                }
                std::thread::sleep(std::time::Duration::from_millis(pause_length));
            }
            icy_engine::MusicAction::Pause(length) => {
                let duration = 250_000 / length;
                std::thread::sleep(std::time::Duration::from_millis(u64::from(duration)));
            }
        }
    }
}

pub fn beep() {
    thread::spawn(|| {
        let stream = stream_setup_for(move |o| {
            o.tick();
            o.tone(800.)
        })
        .unwrap();
        stream.play().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(200));
    });
}

pub struct SampleRequestOptions {
    pub sample_rate: f32,
    pub sample_clock: f32,
    pub nchannels: usize,
}

impl SampleRequestOptions {
    fn tone(&self, freq: f32) -> f32 {
        (self.sample_clock * freq * 2.0 * std::f32::consts::PI / self.sample_rate).sin()
    }
    fn tick(&mut self) {
        self.sample_clock = (self.sample_clock + 1.0) % self.sample_rate;
    }
}

pub fn stream_setup_for<F>(on_sample: F) -> EngineResult<cpal::Stream>
where
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static + Copy,
{
    let (_, device, config) = host_device_setup()?;

    match config.sample_format() {
        cpal::SampleFormat::I8 => stream_make::<i8, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::I16 => stream_make::<i16, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::I32 => stream_make::<i32, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::I64 => stream_make::<i64, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::U8 => stream_make::<u8, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::U16 => stream_make::<u16, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::U32 => stream_make::<u32, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::U64 => stream_make::<u64, _>(&device, &config.into(), on_sample),
        cpal::SampleFormat::F32 => stream_make::<f32, _>(&device, &config.into(), on_sample),
        sample_format => Err(Box::new(io::Error::new(
            ErrorKind::InvalidData,
            format!("Unsupported sample format '{sample_format}'"),
        ))),
    }
}

pub fn host_device_setup() -> EngineResult<(cpal::Host, cpal::Device, cpal::SupportedStreamConfig)>
{
    let host = cpal::default_host();

    let device = host
        .default_output_device()
        .ok_or("Default output device is not available")?;
    let config = device.default_output_config()?;
    Ok((host, device, config))
}

pub fn stream_make<T, F>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    on_sample: F,
) -> EngineResult<cpal::Stream>
where
    T: SizedSample + FromSample<f32>,
    F: FnMut(&mut SampleRequestOptions) -> f32 + std::marker::Send + 'static + Copy,
{
    let sample_rate = config.sample_rate.0 as f32;
    let sample_clock = 0f32;
    let nchannels = config.channels as usize;
    let mut request = SampleRequestOptions {
        sample_rate,
        sample_clock,
        nchannels,
    };
    let err_fn = |err| eprintln!("Error building output sound stream: {err}");
    let mut on_sample = on_sample;
    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
            for frame in output.chunks_mut(request.nchannels) {
                let value: T = T::from_sample(on_sample(&mut request));
                for sample in &mut *frame {
                    *sample = value;
                }
            }
        },
        err_fn,
        None,
    )?;

    Ok(stream)
}
