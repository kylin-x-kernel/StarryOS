// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{
    format,
    string::{String, ToString},
    vec::Vec,
};

use axnet::{
    RecvOptions, SendOptions, SocketAddrEx, SocketOps,
    unix::{StreamTransport, UnixSocket, UnixSocketAddr},
};
use axtask::current;
use bincode::config;
use starry_core::task::{AsThread, Thread};
use tee_raw_sys::{
    TEE_ERROR_GENERIC, TEE_ERROR_ITEM_NOT_FOUND, TEE_ErrorOrigin, TEE_SUCCESS, utee_params,
};

use crate::{
    socket::SocketAddrExt,
    tee::{
        TeeResult,
        protocol::{CARequest, CAResponse, Parameters, TARequest},
        tee_session::{with_tee_ta_ctx, with_tee_ta_ctx_mut},
    },
};

#[derive(Debug, Clone)]
pub struct SessionIdentity {
    pub uuid: String,
    pub session_id: u32,
}

pub fn tee_ta_init_session(uuid: String) -> TeeResult<u32> {
    // Connect to dest TA via Unix socket
    let socket = UnixSocket::new(StreamTransport::new(
        current().as_thread().proc_data.proc.pid(),
    ));
    let path = format!("/tmp/{}.sock", uuid);
    let remote_addr = SocketAddrEx::Unix(UnixSocketAddr::Path(path.into()));
    socket.connect(remote_addr).map_err(|_| TEE_ERROR_GENERIC)?;

    // Send open session request to dest TA
    let req = CARequest::OpenSession {
        params: Parameters::default(),
    };
    let encoded = bincode::encode_to_vec(req, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    let mut message = Vec::with_capacity(4 + encoded.len());
    message.extend_from_slice(&(encoded.len() as u32).to_ne_bytes());
    message.extend_from_slice(&encoded);
    let mut src = message.as_slice();
    socket
        .send(&mut src, SendOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;

    // Receive response from dest TA
    let mut buf = [0u8; 1024];
    let mut dst = buf.as_mut_slice();
    socket
        .recv(&mut dst, RecvOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;
    let (resp, _): (CAResponse, _) =
        bincode::decode_from_slice(&dst, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    match resp {
        CAResponse::OpenSession { status, session_id } => match status {
            TEE_SUCCESS => with_tee_ta_ctx_mut(|ctx| {
                let handle = ctx.session_handle;
                ctx.open_sessions
                    .insert(handle, SessionIdentity { uuid, session_id });
                ctx.session_handle += 1;
                Ok(handle)
            }),
            _ => return Err(status),
        },
        _ => return Err(TEE_ERROR_GENERIC),
    }
}

pub fn tee_ta_close_session(sess_id: SessionIdentity) -> TeeResult {
    // Connect to dest TA via Unix socket
    let socket = UnixSocket::new(StreamTransport::new(
        current().as_thread().proc_data.proc.pid(),
    ));
    let path = format!("/tmp/{}.sock", sess_id.uuid);
    let remote_addr = SocketAddrEx::Unix(UnixSocketAddr::Path(path.into()));
    socket.connect(remote_addr).map_err(|_| TEE_ERROR_GENERIC)?;

    // Send close session request to dest TA
    let req = CARequest::CloseSession {
        session_id: sess_id.session_id,
    };
    let encoded = bincode::encode_to_vec(req, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    let mut message = Vec::with_capacity(4 + encoded.len());
    message.extend_from_slice(&(encoded.len() as u32).to_ne_bytes());
    message.extend_from_slice(&encoded);
    let mut src = message.as_slice();
    socket
        .send(&mut src, SendOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;

    Ok(())
}

pub fn tee_ta_invoke_command(
    sess_id: SessionIdentity,
    cmd_id: u32,
    usr_param: *mut utee_params,
) -> TeeResult {
    // Connect to dest TA via Unix socket
    let socket = UnixSocket::new(StreamTransport::new(
        current().as_thread().proc_data.proc.pid(),
    ));
    let path = format!("/tmp/{}.sock", sess_id.uuid);
    let remote_addr = SocketAddrEx::Unix(UnixSocketAddr::Path(path.into()));
    socket.connect(remote_addr).map_err(|_| TEE_ERROR_GENERIC)?;

    // Send invoke command request to dest TA
    let req = CARequest::InvokeCommand {
        session_id: sess_id.session_id,
        cmd_id,
        params: Parameters::default(),
    };
    let encoded = bincode::encode_to_vec(req, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    let mut message = Vec::with_capacity(4 + encoded.len());
    message.extend_from_slice(&(encoded.len() as u32).to_ne_bytes());
    message.extend_from_slice(&encoded);
    let mut src = message.as_slice();
    socket
        .send(&mut src, SendOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;

    // Receive response from dest TA
    let mut buf = [0u8; 1024];
    let mut dst = buf.as_mut_slice();
    socket
        .recv(&mut dst, RecvOptions::default())
        .map_err(|_| TEE_ERROR_GENERIC)?;
    let (resp, _): (CAResponse, _) =
        bincode::decode_from_slice(&dst, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    match resp {
        CAResponse::InvokeCommand {
            status,
            session_id,
            cmd_id,
            params,
        } => match status {
            TEE_SUCCESS => Ok(()),
            _ => Err(status),
        },
        _ => Err(TEE_ERROR_GENERIC),
    }
}

pub fn tee_ta_get_session(handle: u32) -> TeeResult<SessionIdentity> {
    with_tee_ta_ctx(|ctx| match ctx.open_sessions.get(&handle) {
        Some(sess_id) => Ok(sess_id.clone()),
        None => Err(TEE_ERROR_ITEM_NOT_FOUND),
    })
}

#[cfg(feature = "ta_gprof_support")]
#[derive(Debug)]
pub struct SampleBuf {
    /// Size of samples array in uint16_t
    pub nsamples: u32,
    /// Passed from user mode
    pub offset: u32,
    /// Passed from user mode
    pub scale: u32,
    /// Number of samples taken
    pub count: u32,
    /// Sampling enabled?
    pub enabled: bool,
    /// Samples array
    pub samples: Vec<u16>,
    /// Total user CPU time for this session
    pub usr: u64,
    /// When this session last entered user mode
    pub usr_entered: u64,
    /// usr divided by freq is in seconds
    pub freq: u32,
}

#[cfg(feature = "ta_gprof_support")]
impl SampleBuf {
    pub fn new(nsamples: u32, offset: u32, scale: u32, freq: u32) -> Self {
        Self {
            nsamples,
            offset,
            scale,
            count: 0,
            enabled: false,
            samples: vec![0; nsamples as usize],
            usr: 0,
            usr_entered: 0,
            freq,
        }
    }

    /// Enable sampling
    pub fn enable(&mut self) {
        self.enabled = true;
        self.count = 0;
    }

    /// Disable sampling
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Add a sample to the buffer
    pub fn add_sample(&mut self, value: u16) -> TeeResult {
        if !self.enabled {
            // Sampling is not enabled
            return Err(TEE_ERROR_NOT_SUPPORTED);
        }

        if self.count >= self.nsamples {
            // Sample buffer is full
            return Err(TEE_ERROR_OUT_OF_MEMORY);
        }

        let index = (self.count % self.nsamples) as usize;
        self.samples[index] = value;
        self.count += 1;

        Ok(())
    }

    /// Reset the sample buffer
    pub fn reset(&mut self) {
        self.count = 0;
        self.usr = 0;
        self.usr_entered = 0;
    }

    /// Get the current number of samples
    pub fn get_count(&self) -> u32 {
        self.count
    }

    /// Get the total number of possible samples
    pub fn get_capacity(&self) -> u32 {
        self.nsamples
    }

    /// Get a reference to the samples array
    pub fn get_samples(&self) -> &[u16] {
        &self.samples
    }

    /// Set the time when the session last entered user mode
    pub fn set_usr_entered(&mut self, time: u64) {
        self.usr_entered = time;
    }

    /// Update the total user CPU time
    pub fn update_usr_time(&mut self, additional_time: u64) {
        self.usr += additional_time;
    }

    /// Get the total user CPU time
    pub fn get_usr_time(&self) -> u64 {
        self.usr
    }
}