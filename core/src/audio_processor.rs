use nnnoiseless::DenoiseState;
use ractor::{Actor, ActorProcessingErr, ActorRef};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};
use webrtc_vad;

use crate::config::Settings;
use crate::types::{AudioChunk, AudioProcessorMsg, CoordinatorMsg, TranscriberMsg};

// Message types for VAD thread communication
enum VadRequest {
    ProcessChunk(Vec<i16>, u32), // Samples, sample rate
    SetMode(webrtc_vad::VadMode),
    Shutdown,
}

enum VadResponse {
    Result(bool), // true = voice detected, false = silence
    Error(String),
}

pub struct AudioProcessorActor {}

pub struct AudioProcessorState {
    denoise_state: Option<nnnoiseless::DenoiseState<'static>>,
    vad_sender: Option<Sender<VadRequest>>,
    vad_receiver: Option<Receiver<VadResponse>>,
    coordinator: ActorRef<CoordinatorMsg>,
    transcriber: ActorRef<TranscriberMsg>,
    sample_rate: u32,
    config: Arc<Settings>,
    silence_start: Option<Instant>,
    is_silent: bool,
    frames_since_reporting: usize,
}

// Frame size for VAD processing (30ms at 16kHz = 480 samples)
const VAD_FRAME_SIZE_MS: usize = 30; // 30ms frames

// Spawn a new thread for VAD processing
fn spawn_vad_thread(
    mode: webrtc_vad::VadMode,
) -> Result<(Sender<VadRequest>, Receiver<VadResponse>), String> {
    // Create channels for communication
    let (req_tx, req_rx) = mpsc::channel::<VadRequest>();
    let (resp_tx, resp_rx) = mpsc::channel::<VadResponse>();

    // Spawn thread
    thread::spawn(move || {
        // Initialize VAD - Vad::new() directly returns a Vad instance (not Result<Vad>)
        let mut vad = webrtc_vad::Vad::new();

        // Set initial mode - set_mode() doesn't return a Result
        vad.set_mode(mode);

        // Process requests
        loop {
            match req_rx.recv() {
                Ok(VadRequest::ProcessChunk(samples, _sample_rate)) => {
                    match vad.is_voice_segment(&samples) {
                        Ok(has_voice) => {
                            if let Err(e) = resp_tx.send(VadResponse::Result(has_voice)) {
                                eprintln!("Failed to send VAD result: {:?}", e);
                            }
                        }
                        Err(e) => {
                            if let Err(e) =
                                resp_tx.send(VadResponse::Error(format!("VAD error: {:?}", e)))
                            {
                                eprintln!("Failed to send VAD error: {:?}", e);
                            }
                        }
                    }
                }
                Ok(VadRequest::SetMode(new_mode)) => {
                    // set_mode doesn't return a result
                    vad.set_mode(new_mode);
                }
                Ok(VadRequest::Shutdown) => {
                    break;
                }
                Err(e) => {
                    eprintln!("VAD thread error: {:?}", e);
                    break;
                }
            }
        }

        tracing::info!("VAD thread shutting down");
    });

    Ok((req_tx, resp_rx))
}

#[ractor::async_trait]
impl Actor for AudioProcessorActor {
    type Msg = AudioProcessorMsg;
    type State = AudioProcessorState;
    type Arguments = (
        ActorRef<CoordinatorMsg>,
        ActorRef<TranscriberMsg>,
        u32,
        Arc<Settings>,
    );

    async fn pre_start(
        &self,
        _myself: ActorRef<Self::Msg>,
        (coordinator, transcriber, sample_rate, config): Self::Arguments,
    ) -> Result<Self::State, ActorProcessingErr> {
        tracing::info!("AudioProcessorActor started");

        // Initialize the noise reduction state if enabled
        let denoise_state = if config.enable_denoise {
            // DenoiseState::new() returns a Box<DenoiseState>
            Some(*nnnoiseless::DenoiseState::new())
        } else {
            None
        };

        // Initialize the VAD thread if enabled
        let (vad_sender, vad_receiver) = if config.enable_vad {
            match spawn_vad_thread(config.vad_mode.into()) {
                Ok((sender, receiver)) => {
                    tracing::info!("VAD thread started with mode: {:?}", config.vad_mode);
                    (Some(sender), Some(receiver))
                }
                Err(e) => {
                    tracing::error!("Failed to start VAD thread: {}", e);
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        coordinator
            .send_message(CoordinatorMsg::UpdateStatus(format!(
                "Audio processor initialized (denoise: {}, VAD: {})",
                config.enable_denoise, config.enable_vad
            )))
            .ok();

        Ok(AudioProcessorState {
            denoise_state,
            vad_sender,
            vad_receiver,
            coordinator,
            transcriber,
            sample_rate,
            config,
            silence_start: None,
            is_silent: false,
            frames_since_reporting: 0,
        })
    }

    async fn handle(
        &self,
        _myself: ActorRef<Self::Msg>,
        message: Self::Msg,
        state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        match message {
            AudioProcessorMsg::ProcessChunk(chunk) => {
                // Apply denoising if enabled
                let processed_chunk = if let Some(denoise_state) = &mut state.denoise_state {
                    let mut output = vec![0.0; chunk.0.len()];
                    let _ = denoise_state.process_frame(&mut output, &chunk.0);
                    AudioChunk(output)
                } else {
                    chunk
                };

                // Apply VAD if enabled
                if let (Some(vad_sender), Some(vad_receiver)) =
                    (&state.vad_sender, &state.vad_receiver)
                {
                    // Check if we have enough samples for a VAD frame
                    let samples_per_frame = (state.sample_rate as usize * VAD_FRAME_SIZE_MS) / 1000;

                    if processed_chunk.0.len() >= samples_per_frame {
                        // Convert f32 samples to i16 for VAD
                        let i16_samples: Vec<i16> = processed_chunk
                            .0
                            .iter()
                            .take(samples_per_frame)
                            .map(|&s| (s * 32767.0) as i16)
                            .collect();

                        // Send chunk to VAD thread
                        if let Err(e) = vad_sender
                            .send(VadRequest::ProcessChunk(i16_samples, state.sample_rate))
                        {
                            tracing::error!("Failed to send data to VAD thread: {:?}", e);
                        } else {
                            // Get VAD result
                            match vad_receiver.recv() {
                                Ok(VadResponse::Result(has_voice)) => {
                                    state.frames_since_reporting += 1;

                                    // Update silence tracking
                                    if !has_voice {
                                        // No voice detected
                                        if !state.is_silent {
                                            // First silent frame
                                            state.silence_start = Some(Instant::now());
                                        } else if let Some(silence_start) = state.silence_start {
                                            // Already in silence, check duration
                                            let silence_duration = silence_start.elapsed();
                                            let threshold = Duration::from_millis(
                                                state.config.silence_threshold_ms as u64,
                                            );

                                            if silence_duration > threshold
                                                && state.frames_since_reporting > 10
                                            {
                                                // Silent for too long, notify coordinator
                                                state.coordinator.send_message(
                                                    CoordinatorMsg::SilenceDetected(true),
                                                )?;
                                                state.frames_since_reporting = 0;
                                            }
                                        }
                                        state.is_silent = true;
                                    } else {
                                        // Voice detected
                                        if state.is_silent && state.frames_since_reporting > 10 {
                                            // Transition from silence to voice
                                            state.coordinator.send_message(
                                                CoordinatorMsg::SilenceDetected(false),
                                            )?;
                                            state.frames_since_reporting = 0;
                                        }
                                        state.is_silent = false;
                                        state.silence_start = None;
                                    }
                                }
                                Ok(VadResponse::Error(e)) => {
                                    tracing::error!("VAD error: {}", e);
                                }
                                Err(e) => {
                                    tracing::error!("Failed to receive VAD result: {:?}", e);
                                }
                            }
                        }
                    }
                } else {
                    // Also check for simple energy-based voice detection if no VAD
                    if state.config.enable_vad {
                        let energy = processed_chunk
                            .0
                            .iter()
                            .map(|&sample| sample * sample)
                            .sum::<f32>()
                            / processed_chunk.0.len() as f32;

                        let has_voice = energy > state.config.vad_energy_threshold;

                        state.frames_since_reporting += 1;

                        // Update silence tracking
                        if !has_voice {
                            // No voice detected
                            if !state.is_silent {
                                // First silent frame
                                state.silence_start = Some(Instant::now());
                            } else if let Some(silence_start) = state.silence_start {
                                // Already in silence, check duration
                                let silence_duration = silence_start.elapsed();
                                let threshold =
                                    Duration::from_millis(state.config.silence_threshold_ms as u64);

                                if silence_duration > threshold && state.frames_since_reporting > 10
                                {
                                    // Silent for too long, notify coordinator
                                    state
                                        .coordinator
                                        .send_message(CoordinatorMsg::SilenceDetected(true))?;
                                    state.frames_since_reporting = 0;
                                }
                            }
                            state.is_silent = true;
                        } else {
                            // Voice detected
                            if state.is_silent && state.frames_since_reporting > 10 {
                                // Transition from silence to voice
                                state
                                    .coordinator
                                    .send_message(CoordinatorMsg::SilenceDetected(false))?;
                                state.frames_since_reporting = 0;
                            }
                            state.is_silent = false;
                            state.silence_start = None;
                        }
                    }
                }

                // Forward the processed audio to the transcriber
                state
                    .transcriber
                    .send_message(TranscriberMsg::ProcessAudioChunk(processed_chunk))?;
            }
            AudioProcessorMsg::Shutdown => {
                tracing::info!("AudioProcessorActor shutting down");

                // Shutdown VAD thread if running
                if let Some(vad_sender) = &state.vad_sender {
                    let _ = vad_sender.send(VadRequest::Shutdown);
                    tracing::info!("Sent shutdown signal to VAD thread");
                }
            }
        }
        Ok(())
    }

    async fn post_stop(
        &self,
        _myself: ActorRef<Self::Msg>,
        _state: &mut Self::State,
    ) -> Result<(), ActorProcessingErr> {
        tracing::info!("AudioProcessorActor stopped");
        Ok(())
    }
}
