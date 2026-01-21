// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{format, string::String, vec::Vec};

use axnet::{
    RecvOptions, SendOptions, SocketAddrEx, SocketOps,
    unix::{StreamTransport, UnixSocket, UnixSocketAddr},
};
use axtask::current;
use bincode::config;
use starry_core::task::AsThread;
use tee_raw_sys::{TEE_ERROR_GENERIC, TEE_ERROR_ITEM_NOT_FOUND, TEE_SUCCESS, utee_params};

use crate::tee::{
    TeeResult,
    protocal::{Parameters, TeeRequest, TeeResponse},
    tee_session::{with_tee_ta_ctx, with_tee_ta_ctx_mut},
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
    let req = TeeRequest::OpenSession {
        params: Parameters::default(),
        uuid: uuid.clone(),
        connection_method: 0,
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
    let (resp, _): (TeeResponse, _) =
        bincode::decode_from_slice(&dst, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    match resp {
        TeeResponse::OpenSession { session_id, result } => match result {
            TEE_SUCCESS => with_tee_ta_ctx_mut(|ctx| {
                let handle = ctx.session_handle;
                ctx.open_sessions
                    .insert(handle, SessionIdentity { uuid, session_id });
                ctx.session_handle += 1;
                Ok(handle)
            }),
            _ => return Err(result),
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
    let req = TeeRequest::CloseSession {
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
    let req = TeeRequest::InvokeCommand {
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
    let (resp, _): (TeeResponse, _) =
        bincode::decode_from_slice(&dst, config::standard()).map_err(|_| TEE_ERROR_GENERIC)?;
    match resp {
        TeeResponse::InvokeCommand { params, result } => match result {
            TEE_SUCCESS => Ok(()),
            _ => Err(result),
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
