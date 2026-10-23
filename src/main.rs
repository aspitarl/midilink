#[cfg(not(any(unix, windows)))]
compile_error!("MIDILink currently supports Unix-like platforms and Windows");

mod adapter;

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use midir::{MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use tracing::{info, level_filters::LevelFilter, warn};
use tracing_subscriber::EnvFilter;

use crate::adapter::LinkAdapter;

#[derive(Parser)]
#[command(version)]
struct Args {
    /// The name of the virtual MIDI ports, or the existing Windows MIDI port to connect to.
    #[arg(short, long, default_value = "Link")]
    name: String,
}

fn handle_midi_input(stamp: u64, raw: &[u8], adapter: &mut LinkAdapter) {
    if let Err(e) = adapter.handle_raw_event(stamp, raw) {
        warn!(%e, "Error while handling event");
    }
}

#[cfg(unix)]
fn connect_midi_input(
    name: &str,
    adapter: LinkAdapter,
) -> Result<MidiInputConnection<LinkAdapter>> {
    use midir::os::unix::VirtualInput;

    let midi_in = MidiInput::new("MIDILink input")?;
    midi_in
        .create_virtual(name, handle_midi_input, adapter)
        .map_err(|e| anyhow!("Could not create virtual input: {}", e))
}

#[cfg(windows)]
fn connect_midi_input(
    name: &str,
    adapter: LinkAdapter,
) -> Result<MidiInputConnection<LinkAdapter>> {
    let midi_in = MidiInput::new("MIDILink input")?;
    let ports = midi_in.ports();
    let mut available = Vec::with_capacity(ports.len());
    let mut selected = None;

    for port in &ports {
        let port_name = midi_in
            .port_name(port)
            .context("Could not read MIDI input port name")?;
        if port_name == name {
            selected = Some(port.clone());
        }
        available.push(port_name);
    }

    let port = selected.ok_or_else(|| {
        anyhow!(
            "Could not find MIDI input port named {:?}. Available input ports: {}. Create a matching port in loopMIDI or specify its name with --name.",
            name,
            if available.is_empty() { "(none)".to_owned() } else { available.join(", ") }
        )
    })?;

    midi_in
        .connect(&port, "MIDILink input", handle_midi_input, adapter)
        .map_err(|e| anyhow!("Could not connect to MIDI input: {}", e))
}

#[cfg(unix)]
fn connect_midi_output(name: &str) -> Result<MidiOutputConnection> {
    use midir::os::unix::VirtualOutput;

    MidiOutput::new("MIDILink output")?
        .create_virtual(name)
        .map_err(|e| anyhow!("Could not create virtual output: {}", e))
}

#[cfg(windows)]
fn connect_midi_output(name: &str) -> Result<MidiOutputConnection> {
    let midi_out = MidiOutput::new("MIDILink output")?;
    let ports = midi_out.ports();
    let mut available = Vec::with_capacity(ports.len());
    let mut selected = None;

    for port in &ports {
        let port_name = midi_out
            .port_name(port)
            .context("Could not read MIDI output port name")?;
        if port_name == name {
            selected = Some(port.clone());
        }
        available.push(port_name);
    }

    let port = selected.ok_or_else(|| {
        anyhow!(
            "Could not find MIDI output port named {:?}. Available output ports: {}. Create a matching port in loopMIDI or specify its name with --name.",
            name,
            if available.is_empty() { "(none)".to_owned() } else { available.join(", ") }
        )
    })?;

    midi_out
        .connect(&port, "MIDILink output")
        .map_err(|e| anyhow!("Could not connect to MIDI output: {}", e))
}

fn main() -> Result<()> {
    _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::builder()
                .with_default_directive(LevelFilter::INFO.into())
                .from_env()?,
        )
        .init();

    let args = Args::parse();
    let adapter = LinkAdapter::new();

    #[cfg(unix)]
    info!(name = %args.name, "Creating virtual MIDI ports");
    #[cfg(windows)]
    info!(name = %args.name, "Connecting to MIDI ports");

    let _conn_in = connect_midi_input(&args.name, adapter)?;
    let _conn_out = connect_midi_output(&args.name)?;

    info!("Waiting for input...");

    let running = Arc::new(AtomicBool::new(true));

    {
        let running = running.clone();
        let main_thread = thread::current();
        ctrlc::set_handler(move || {
            running.store(false, Ordering::SeqCst);
            main_thread.unpark();
        })?;
    }

    while running.load(Ordering::SeqCst) {
        thread::park();
    }

    info!("Exiting...");
    Ok(())
}
