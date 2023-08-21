use std::{collections::VecDeque, sync::mpsc};

use icy_engine::ansi::sound::{AnsiMusic, MusicAction, MusicStyle};
use rodio::OutputStream;
use web_time::Instant;

use crate::TerminalResult;

use super::Rng;

pub struct SoundThreadData {
    tx: mpsc::Sender<SoundData>,
    rx: mpsc::Receiver<SoundData>,
    thread_is_running: bool,

    music: VecDeque<SoundData>,
}

/// Data that is sent to the connection thread
#[derive(Debug)]
pub enum SoundData {
    PlayMusic(AnsiMusic),
    Beep,
    Clear,

    StartPlay,
    StopPlay,
}

pub struct SoundThread {
    rx: mpsc::Receiver<SoundData>,
    tx: mpsc::Sender<SoundData>,
    is_playing: bool,
    rng: Rng,
    pub stop_button: u32,
    last_stop_cycle: Instant,
}

impl SoundThread {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel::<SoundData>();
        let (tx2, rx2) = mpsc::channel::<SoundData>();

        let mut data = SoundThreadData {
            rx,
            tx: tx2,
            music: VecDeque::new(),
            thread_is_running: true,
        };
        #[cfg(not(target_arch = "wasm32"))]
        std::thread::spawn(move || {
            while data.thread_is_running {
                data.handle_queue();
                data.handle_receive();
            }
            log::error!(
                "communication thread closed because it lost connection with the ui thread."
            );
        });
        let mut rng = Rng::default();
        let stop_button = rng.gen_range(0..6);
        SoundThread {
            rx: rx2,
            tx,
            is_playing: false,
            stop_button,
            rng,
            last_stop_cycle: Instant::now(),
        }
    }

    pub(crate) fn play_music(&self, music: &AnsiMusic) {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.tx.send(SoundData::PlayMusic(music.clone()));
        #[cfg(target_arch = "wasm32")]
        SoundThread::play_music_wasm(music);
    }

    pub(crate) fn beep(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = self.tx.send(SoundData::Beep);
        #[cfg(target_arch = "wasm32")]
        SoundThread::beep_wasm();
    }

    pub(crate) fn clear(&self) {
        let _ = self.tx.send(SoundData::Clear);
    }

    #[cfg(target_arch = "wasm32")]
    pub(crate) fn update_state(&mut self) -> TerminalResult<()> {
        Ok(())
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn update_state(&mut self) -> TerminalResult<()> {
        if self.last_stop_cycle.elapsed().as_secs() > 5 {
            self.stop_button = self.rng.gen_range(0..6);
            self.last_stop_cycle = Instant::now();
        }
        loop {
            match self.rx.try_recv() {
                Ok(data) => match data {
                    SoundData::StartPlay => self.is_playing = true,
                    SoundData::StopPlay => self.is_playing = false,
                    _ => {}
                },

                Err(err) => match err {
                    mpsc::TryRecvError::Empty => break,
                    mpsc::TryRecvError::Disconnected => {
                        return Err(Box::new(err));
                    }
                },
            }
        }
        Ok(())
    }

    pub(crate) fn is_playing(&self) -> bool {
        self.is_playing
    }

    #[cfg(target_arch = "wasm32")]
    fn beep_wasm() {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(0.1);

        let source = rodio::source::SineWave::new(880.);
        sink.append(source);

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    #[cfg(target_arch = "wasm32")]
    fn play_music_wasm(music: &AnsiMusic) {
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
}

impl SoundThreadData {
    pub fn handle_receive(&mut self) -> bool {
        let mut result = false;
        while let Ok(data) = self.rx.try_recv() {
            match data {
                SoundData::PlayMusic(m) => self.music.push_back(SoundData::PlayMusic(m)),
                SoundData::Beep => self.music.push_back(SoundData::Beep),
                SoundData::Clear => {
                    result = true;
                    self.music.clear();
                }
                _ => {}
            }
        }
        result
    }

    fn handle_queue(&mut self) {
        let Some(data) = self.music.pop_front() else {
            return;
        };
        match data {
            SoundData::PlayMusic(music) => self.play_music(&music),
            SoundData::Beep => SoundThreadData::beep(),
            _ => {}
        }
    }

    fn beep() {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(0.1);

        let source = rodio::source::SineWave::new(880.);
        sink.append(source);

        std::thread::sleep(std::time::Duration::from_millis(200));
    }

    fn play_music(&mut self, music: &AnsiMusic) {
        let _ = self.tx.send(SoundData::StartPlay);
        let mut i = 0;
        let mut cur_style = MusicStyle::Normal;

        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = rodio::Sink::try_new(&stream_handle).unwrap();
        sink.set_volume(0.1);

        while i < music.music_actions.len() {
            let act = &music.music_actions[i];
            i += 1;
            if self.handle_receive() {
                break;
            }
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
        let _ = self.tx.send(SoundData::StopPlay);
    }
}
