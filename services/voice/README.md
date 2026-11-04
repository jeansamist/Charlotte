# voice

> **Status: planned, not implemented.** Design from [`docs/architecture.md`](../../docs/architecture.md). Update this file when work starts.

**Tech:** Python

## Role
The voice loop: hears the assistant's name, turns speech into text, and speaks the replies.

## Planned design
- **Wake word + VAD:** local and always on (openWakeWord trained on the configured name). Audio is never streamed
  to the cloud before the wake word.
- **STT / TTS:** cloud adapters (Amazon Transcribe and Polly by default; ElevenLabs, Deepgram or OpenAI as alternatives).
- **Offline fallback:** faster-whisper (STT) and Piper (TTS), used only when the cloud is unreachable.

## Interfaces
- Microphone stream and TTS playback through `services/audio-server` (a new "assistant" input with ducking priority).
- Publishes `voice.wake`, `voice.transcript`. Consumes `brain.reply`.

## Depends on
audio-server, NATS, cloud voice provider credentials.
