use rodio::cpal;
use rodio::cpal::traits::{DeviceTrait, HostTrait};
use rodio::{Decoder, Device, Devices, OutputStream, OutputStreamHandle, Sink};
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
    SetDevice(String),
}

// Add this implementation after the Boombox struct definition
impl Clone for Boombox {
    fn clone(&self) -> Self {
        // Create a new channel for commands
        let (tx, rx) = mpsc::channel();

        // Clone the existing command sender to the new one
        let command_tx = self.command_tx.clone();

        // Spawn a new thread that forwards commands from the new channel to the original one
        let thread_handle = thread::spawn(move || {
            while let Ok(cmd) = rx.recv() {
                if command_tx.send(cmd).is_err() {
                    break;
                }
            }
        });

        Boombox {
            command_tx: tx,
            _thread_handle: thread_handle,
        }
    }
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

    pub fn list_devices(&self) -> Result<Vec<String>, String> {
        let host = cpal::default_host();
        let devices = host
            .output_devices()
            .map_err(|e| format!("Failed to list devices: {}", e))?;
        let device_names: Vec<String> = devices
            .map(|device| {
                device
                    .name()
                    .unwrap_or_else(|_| "Unknown Device".to_string())
            })
            .collect();
        Ok(device_names)
    }

    pub fn set_device(&self, device_name: String) -> Result<(), String> {
        self.command_tx
            .send(AudioCommand::SetDevice(device_name))
            .map_err(|_| "Failed to send set_device command to audio thread".to_string())
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
    _stream: Option<OutputStream>,
    stream_handle: Option<OutputStreamHandle>,
    sink: Option<Sink>,
    current_device: String,
}

impl AudioThread {
    fn new() -> Result<Self, String> {
        let (stream, stream_handle) = OutputStream::try_default()
            .map_err(|e| format!("Failed to get default output device: {}", e))?;

        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("No default output device found")?;
        let default_device_name = device
            .name()
            .map_err(|e| format!("Failed to get default device name: {}", e))?;

        Ok(AudioThread {
            _stream: Some(stream),
            stream_handle: Some(stream_handle),
            sink: None,
            current_device: default_device_name,
        })
    }

    fn create_sink(&mut self) -> Result<(), String> {
        // Ensure we have a valid stream handle
        let stream_handle = self
            .stream_handle
            .as_ref()
            .ok_or("No stream handle available")?;

        let sink = Sink::try_new(stream_handle)
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

    fn set_device(&mut self, device_name: String) -> Result<(), String> {
        // Stop the current playback
        if let Some(sink) = &mut self.sink {
            sink.stop();
        }

        // Try to open the specified device
        let host = cpal::default_host();
        let devices = host
            .output_devices()
            .map_err(|e| format!("Failed to list devices: {}", e))?;

        let mut found_device = false;
        for device in devices {
            if device
                .name()
                .map_err(|e| format!("Failed to get device name: {}", e))?
                == device_name
            {
                found_device = true;
                // Device found, create a new stream and stream handle
                let (new_stream, new_stream_handle) = OutputStream::try_from_device(&device)
                    .map_err(|e| format!("Failed to open device {}: {}", device_name, e))?;

                self._stream = Some(new_stream);
                self.stream_handle = Some(new_stream_handle);
                self.current_device = device_name.clone();

                // Create a new sink for the new device
                self.create_sink()?;
                break;
            }
        }

        if !found_device {
            return Err(format!("Device {} not found", device_name));
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
                    AudioCommand::SetDevice(device_name) => {
                        let _ = self.set_device(device_name);
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
