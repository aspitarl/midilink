# MIDILink

[![crates.io](https://img.shields.io/crates/v/midilink)](https://crates.io/crates/midilink)
[![Build](https://github.com/fwcd/midilink/actions/workflows/build.yml/badge.svg)](https://github.com/fwcd/midilink/actions/workflows/build.yml)

Mixxx MIDI to Ableton Link adapter. Exports tempo and phase from Mixxx to apps and devices implementing the Ableton Link protocol (e.g. lighting, instruments, DAWs).

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
