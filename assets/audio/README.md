# Startup and wake sound

`terasawa-mukumo-startup.wav` is embedded into the device binary at compile time.
It plays when muku starts, when the Linux device resumes from suspend, and when
muku wakes from its in-app sleep state.

The source M4A supplied for this project was converted to 22.05kHz mono,
16-bit PCM and normalized to -16 LUFS with a -1.5dB peak limit for the
handheld speaker.
