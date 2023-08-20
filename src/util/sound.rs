use std::thread;

use icy_engine::ansi::sound::{AnsiMusic, MusicAction, MusicStyle};
use rodio::OutputStream;

pub fn play_music(music: &AnsiMusic) {
    let mut i = 0;
    let mut cur_style = MusicStyle::Normal;

    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let sink = rodio::Sink::try_new(&stream_handle).unwrap();
    sink.set_volume(0.1);

    while i < music.music_actions.len() {
        let act = &music.music_actions[i];
        i += 1;
        match act {
            MusicAction::SetStyle(style) => {
                cur_style = *style;
            }
            MusicAction::PlayNote(freq, length, dotted) => {
                let f = *freq;

                let mut duration =
                    if *dotted { 420_000_u64 } else { 300_000_u64 } / u64::from(*length);

                let pause_length = match cur_style {
                    MusicStyle::Legato => 0,
                    MusicStyle::Staccato => duration / 4,
                    _ => duration / 8,
                };
                duration -= pause_length;
                {
                    let source = rodio::source::SineWave::new(f);
                    sink.append(source);
                    sink.play();
                    std::thread::sleep(std::time::Duration::from_millis(duration));
                    sink.clear();
                }
                std::thread::sleep(std::time::Duration::from_millis(pause_length));
            }
            MusicAction::Pause(length) => {
                let duration = 2 * 250_000 / length;
                std::thread::sleep(std::time::Duration::from_millis(u64::from(duration)));
            }
        }
    }
}

pub fn beep() {
    thread::spawn(|| {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(0.1);

        let source = rodio::source::SineWave::new(880.);
        sink.append(source);

        std::thread::sleep(std::time::Duration::from_millis(200));
    });
}
