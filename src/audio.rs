//! Non-blocking WAV playback for webhook TTS messages.

use std::{
    fs,
    process::Command,
    sync::mpsc::{self, SyncSender, TrySendError},
    thread,
};

const AUDIO_QUEUE_DEPTH: usize = 4;

pub struct AudioPlayer {
    sender: SyncSender<Vec<u8>>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(AUDIO_QUEUE_DEPTH);
        thread::spawn(move || {
            let path = std::env::temp_dir().join(format!("muku-tts-{}.wav", std::process::id()));
            while let Ok(bytes) = receiver.recv() {
                if let Err(error) = fs::write(&path, bytes) {
                    eprintln!("audio write failed: {error}");
                    continue;
                }
                match Command::new("aplay").arg("-q").arg(&path).status() {
                    Ok(status) if status.success() => {}
                    Ok(status) => eprintln!("aplay exited with {status}"),
                    Err(error) => eprintln!("aplay failed: {error}"),
                }
                if let Err(error) = fs::remove_file(&path) {
                    eprintln!("audio cleanup failed: {error}");
                }
            }
        });
        Self { sender }
    }

    pub fn play(&self, wav: Vec<u8>) {
        match self.sender.try_send(wav) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => eprintln!("audio queue is full; notification skipped"),
            Err(TrySendError::Disconnected(_)) => eprintln!("audio player stopped"),
        }
    }
}
