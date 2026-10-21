//! Non-blocking WAV playback for startup sounds and webhook messages.

use std::{
    fs,
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::{self, SyncSender, TrySendError},
        Arc,
    },
    thread,
    time::Duration,
};

const AUDIO_QUEUE_DEPTH: usize = 4;

pub struct AudioPlayer {
    sender: SyncSender<Vec<u8>>,
    stop: Arc<AtomicBool>,
}

impl AudioPlayer {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::sync_channel::<Vec<u8>>(AUDIO_QUEUE_DEPTH);
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        thread::spawn(move || {
            let path = std::env::temp_dir().join(format!("muku-audio-{}.wav", std::process::id()));
            while let Ok(bytes) = receiver.recv() {
                if let Err(error) = fs::write(&path, bytes) {
                    eprintln!("audio write failed: {error}");
                    continue;
                }
                match Command::new("aplay").arg("-q").arg(&path).spawn() {
                    Ok(mut child) => {
                        let result = loop {
                            match child.try_wait() {
                                Ok(Some(status)) => break Some(status),
                                Ok(None) if worker_stop.load(Ordering::Relaxed) => {
                                    let _ = child.kill();
                                    let _ = child.wait();
                                    break None;
                                }
                                Ok(None) => thread::sleep(Duration::from_millis(25)),
                                Err(error) => {
                                    eprintln!("aplay wait failed: {error}");
                                    let _ = child.kill();
                                    let _ = child.wait();
                                    break None;
                                }
                            }
                        };
                        if let Some(status) = result {
                            if !status.success() {
                                eprintln!("aplay exited with {status}");
                            }
                        }
                    }
                    Err(error) => eprintln!("aplay failed: {error}"),
                }
                if let Err(error) = fs::remove_file(&path) {
                    eprintln!("audio cleanup failed: {error}");
                }
                if worker_stop.load(Ordering::Relaxed) {
                    break;
                }
            }
        });
        Self { sender, stop }
    }

    pub fn play(&self, wav: Vec<u8>) {
        match self.sender.try_send(wav) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => eprintln!("audio queue is full; sound skipped"),
            Err(TrySendError::Disconnected(_)) => eprintln!("audio player stopped"),
        }
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}
