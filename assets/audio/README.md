# Startup and wake sound

`terasawa-mukumo-startup.wav` is embedded into the device binary at compile time.
It plays when muku starts, when the Linux device resumes from suspend, and when
muku wakes from its in-app sleep state.

The source M4A supplied for this project is converted to 48kHz mono, 16-bit
PCM without time stretching or loudness normalization. The 48kHz rate matches
the handhelds' audio output path, so speech keeps its original pitch and tempo.
