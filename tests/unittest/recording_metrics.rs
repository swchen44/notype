//! Native public recorder contract with a real consumer/resampler/VAD and owned
//! hardware channel/ring fixture. It does not open a device or read user data.
use super::*;
struct SilenceVad;
impl VoiceActivityDetector for SilenceVad {
    fn push_frame<'a>(&'a mut self, _: &'a [f32]) -> anyhow::Result<VadFrame<'a>> {
        Ok(VadFrame::Noise)
    }
    fn frame_samples(&self) -> usize {
        480
    }
}
fn capture(policy: VadPolicy, dropped: u64) -> CapturedAudio {
    let vad = VadConfig {
        detector: Arc::new(Mutex::new(Box::new(SilenceVad))),
        frame_samples: 480,
        offline_hangover_frames: 0,
        streaming_hangover_frames: 0,
    };
    let processor = CaptureProcessor::new(48000, Some(vad), None, None, Instant::now());
    let (mut producer, consumer) = RingBuffer::new(48000);
    let transport = Arc::new(CaptureTransportState::default());
    AudioRecorder::write_input_to_ring(&[0.0; 24000], 1, None, &mut producer, &transport);
    let (sender, receiver) = mpsc::channel();
    let mut recorder = AudioRecorder::new().unwrap();
    recorder.cmd_tx = Some(sender);
    let ready = recorder.start(policy).unwrap();
    let observed = Arc::clone(&transport);
    recorder.worker_handle = Some(std::thread::spawn(move || {
        run_consumer(
            processor,
            consumer,
            receiver,
            observed,
            Arc::new(AtomicBool::new(false)),
        )
    }));
    ready.recv_timeout(Duration::from_secs(1)).unwrap();
    transport.overrun_samples.store(dropped, Ordering::Release);
    let boundary = Arc::clone(&transport);
    let hardware = std::thread::spawn(move || {
        let start = Instant::now();
        while !boundary.pause_requested.load(Ordering::Acquire)
            && start.elapsed() < Duration::from_secs(1)
        {
            std::thread::yield_now();
        }
        assert!(boundary.pause_requested.load(Ordering::Acquire));
        boundary.pause_acknowledged.store(true, Ordering::Release);
    });
    let captured = recorder.stop_observed().unwrap();
    hardware.join().unwrap();
    recorder.close().unwrap();
    captured
}
#[test]
fn public_stop_keeps_received_silence_when_vad_discards_decoder_input() {
    let captured = capture(VadPolicy::Offline, 480).with_decoder_padding();
    assert!(captured.samples.is_empty());
    let observation = captured.observation.unwrap();
    assert_eq!(observation.audio_ms(), 500);
    assert_eq!(observation.quality(), "incomplete");
}
#[test]
fn public_stop_padding_cannot_inflate_native_received_duration() {
    let captured = capture(VadPolicy::Disabled, 0).with_decoder_padding();
    assert_eq!(captured.samples.len(), 20000);
    let observation = captured.observation.unwrap();
    assert_eq!(observation.received_frames, 24000);
    assert_eq!(observation.audio_ms(), 500);
    assert_eq!(observation.quality(), "complete");
}
