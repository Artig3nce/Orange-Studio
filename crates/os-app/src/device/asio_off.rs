// SPDX-License-Identifier: GPL-3.0-or-later

//! A Windows build without ASIO (the `asio` feature off, as a MinGW cross-build must be): no drivers to list or open.

use super::{Errors, Mix};

pub fn drivers() -> Vec<String> {
    Vec::new()
}

/// Never made: nothing opens.
pub(crate) struct AsioOut;

pub(crate) struct Opened {
    pub(crate) out: AsioOut,
    pub(crate) rate: u32,
    /// Its buffer length, and the lengths it takes.
    pub(crate) frames: u32,
    pub(crate) buffers: (u32, u32),
    pub(crate) channels: u16,
    /// Inputs open (0: none, or the driver refused them).
    pub(crate) inputs: u16,
    /// The driver's names for the outputs and inputs open.
    pub(crate) out_names: Vec<String>,
    pub(crate) in_names: Vec<String>,
    /// The rates of `audio_cfg::RATES` and the lengths of `audio_cfg::BUFFERS` it takes.
    pub(crate) rates: Vec<u32>,
    pub(crate) sizes: Vec<u32>,
    /// Its output and input latencies, frames (`ASIOGetLatencies`).
    pub(crate) latency: Option<u32>,
    pub(crate) input_latency: Option<u32>,
}

pub(crate) fn open(_name: &str, _rate: Option<u32>, _buffer: Option<u32>, _mix: Mix, _errors: Errors) -> Result<Opened, String> {
    Err("this build has no ASIO".into())
}

pub(crate) fn control_panel() -> Result<(), String> {
    Err("this build has no ASIO".into())
}
