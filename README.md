# MIDILink

[![crates.io](https://img.shields.io/crates/v/midilink)](https://crates.io/crates/midilink)
[![Build](https://github.com/fwcd/midilink/actions/workflows/build.yml/badge.svg)](https://github.com/fwcd/midilink/actions/workflows/build.yml)

Mixxx MIDI to Ableton Link adapter. Exports tempo and phase from Mixxx to apps and devices implementing the Ableton Link protocol (e.g. lighting, instruments, DAWs).

## Bars and downbeats

Mixxx only sends beat pulses, not bar positions. MIDILink therefore counts beats
and exposes a Link quantum of 4 by default (change it with `--quantum`, e.g.
`--quantum 8` for phrases, or `--quantum 1` for single beats). The first beat
after playback starts or restarts is treated as the downbeat. To realign while
playing, press Enter in the MIDILink terminal on the first beat of a bar.

## Windows

MIDILink can use an existing MIDI port on Windows. Create a port in
[loopMIDI](https://www.tobias-erichsen.de/software/loopmidi.html) named `Link`,
then start MIDILink. It connects to the matching input and output ports. If your
loopMIDI port has a different name, pass it with `--name`, for example
`midilink --name "Mixxx Link"`.

Configure Mixxx to send its MIDI clock to the matching loopMIDI port. The port
must exist before starting MIDILink.

To build from source, install LLVM and set `LIBCLANG_PATH` to its `bin`
directory (for example, `setx LIBCLANG_PATH "C:\Program Files\LLVM\bin"`),
then open a new terminal. The build also requires the Visual Studio C++ build
tools and CMake.
