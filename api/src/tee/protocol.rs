// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been created by KylinSoft on 2025.

use alloc::{string::String, vec::Vec};

use bincode::{Decode, Encode};

#[derive(Encode, Decode)]
pub enum TARequest {
    Register { uuid: String },
    OpenSession { uuid: String, params: Parameters },
}

#[derive(Encode, Decode)]
pub enum CARequest {
    OpenSession {
        params: Parameters,
    },
    CloseSession {
        session_id: u32,
    },
    Return {
        ret: u32,
    },
    Destroy,
    InvokeCommand {
        session_id: u32,
        cmd_id: u32,
        params: Parameters,
    },
}

#[derive(Encode, Decode)]
pub enum CAResponse {
    OpenSession {
        status: u32,
        session_id: u32,
    },
    CloseSession {
        status: u32,
        session_id: u32,
    },
    Return {
        status: u32,
    },
    Destroy {
        status: u32,
    },
    InvokeCommand {
        status: u32,
        session_id: u32,
        cmd_id: u32,
        params: Parameters,
    },
}

#[derive(Encode, Decode)]
pub struct Parameters(pub Parameter, pub Parameter, pub Parameter, pub Parameter);

impl Parameters {
    pub fn default() -> Self {
        Parameters(
            Parameter::default(),
            Parameter::default(),
            Parameter::default(),
            Parameter::default(),
        )
    }
}

#[derive(Encode, Decode)]
pub struct Parameter {
    pub raw: TEEParam,
    pub param_type: ParamType,
}

impl Parameter {
    pub fn default() -> Self {
        Parameter {
            raw: TEEParam {
                data: Vec::new(),
                value: Value { a: 0, b: 0 },
            },
            param_type: ParamType::None,
        }
    }
}

#[derive(Encode, Decode)]
pub struct TEEParam {
    pub data: Vec<u8>,
    pub value: Value,
}

#[derive(Encode, Decode, Clone, Copy)]
pub struct Value {
    pub a: u32,
    pub b: u32,
}

#[derive(Encode, Decode)]
pub enum ParamType {
    None         = 0,
    ValueInput   = 1,
    ValueOutput  = 2,
    ValueInout   = 3,
    MemrefInput  = 5,
    MemrefOutput = 6,
    MemrefInout  = 7,
}

impl From<u32> for ParamType {
    fn from(value: u32) -> Self {
        match value {
            0 => ParamType::None,
            1 => ParamType::ValueInput,
            2 => ParamType::ValueOutput,
            3 => ParamType::ValueInout,
            5 => ParamType::MemrefInput,
            6 => ParamType::MemrefOutput,
            7 => ParamType::MemrefInout,
            _ => ParamType::None,
        }
    }
}
