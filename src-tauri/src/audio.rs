use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

// Commands that can be sent to the audio thread
enum AudioCommand {
    Play(String),
    Pause,
    Resume,
    Stop,
    SetVolume(f32),
    Shutdown,
}

pub struct Boombox {
    command_tx: Sender<AudioCommand>,
    _thread_handle: thread::JoinHandle<()>,
}

impl Boombox {
    pub fn new() -> Result<Self, String> {
        // Create a channel for communication
        let (tx, rx) = mpsc::channel::<AudioCommand>();

        // Spawn a new thread for audio processing
        let thread_handle = thread::spawn(move || {
            let mut audio_thread = AudioThread::new().expect("Failed to initialize audio thread");
            audio_thread.run(rx);
        });

        Ok(Boombox {
            command_tx: tx,
            _thread_handle: thread_handle,
        })
    }

    pub fn play_file(&self, path: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Play(path))
            .map_err(|_| "Failed to send play command to audio thread".to_string())
    }

    pub fn pause(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Pause)
            .map_err(|_| "Failed to send pause command to audio thread".to_string())
    }

    pub fn resume(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Resume)
            .map_err(|_| "Failed to send resume command to audio thread".to_string())
    }

    pub fn stop(&self) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::Stop)
            .map_err(|_| "Failed to send stop command to audio thread".to_string())
    }

    pub fn set_volume(&self, volume: f32) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetVolume(volume))
            .map_err(|_| "Failed to send volume command to audio thread".to_string())
    }
}

impl Drop for Boombox {
    fn drop(&mut self) {
        // Try to send shutdown command when Boombox is dropped
        let _ = self.command_tx.send(AudioCommand::Shutdown);
    }
}

// The actual audio processing happens in this struct
struct AudioThread {
    _stream: OutputStream,
    stream_handle: OutputStreamHandle,
    sink: Option<Sink>,
}

impl AudioThread {
    fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to get default output device: {}", e))?;

        Ok(AudioThread {
            _stream: stream,
            stream_handle,
            sink: None,
        })
    }

    fn create_sink(&mut self) -> Result<(), String> {
        let sink = Sink::try_new(&self.stream_handle)
            .map_err(|e| format!("Failed to create audio sink: {}", e))?;
        self.sink = Some(sink);
        Ok(())
    }

    fn play_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), String> {
        // Create a new sink for each playback
        self.create_sink()?;

        let file = File::open(path).map_err(|e| format!("Failed to open audio file: {}", e))?;

        let source = Decoder::new(BufReader::new(file))
            .map_err(|e| format!("Failed to decode audio file: {}", e))?;

        if let Some(sink) = &self.sink {
            sink.append(source);
            sink.play();
        }

        Ok(())
    }

    fn run(&mut self, rx: Receiver<AudioCommand>) {
        loop {
            match rx.recv() {
                Ok(command) => match command {
                    AudioCommand::Play(path) => {
                        let _ = self.play_file(path);
                    }
                    AudioCommand::Pause => {
                        if let Some(sink) = &self.sink {
                            sink.pause();
                        }
                    }
                    AudioCommand::Resume => {
                        if let Some(sink) = &self.sink {
                            sink.play();
                        }
                    }
                    AudioCommand::Stop => {
                        if let Some(sink) = &self.sink {
                            sink.stop();
                        }
                    }
                    AudioCommand::SetVolume(volume) => {
                        if let Some(sink) = &self.sink {
                            sink.set_volume(volume);
                        }
                    }
                    AudioCommand::Shutdown => {
                        break;
                    }
                },
                Err(_) => {
                    // Channel is closed, exit the thread
                    break;
                }
            }
        }
    }
}
