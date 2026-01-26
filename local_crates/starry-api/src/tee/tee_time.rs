// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.
//
// This file has been modified by KylinSoft on 2025.
//

use alloc::vec::Vec;

use axhal::time::{TimeValue, wall_time};
use tee_raw_sys::{
    TEE_ERROR_BAD_PARAMETERS, TEE_ERROR_OVERFLOW, TEE_ERROR_TIME_NOT_SET, TEE_UUID, TeeTime,
};

use crate::tee::{
    TeeResult,
    tee_session::{with_tee_session_ctx, with_tee_session_ctx_mut},
    user_access::{copy_from_user_struct, copy_to_user_struct},
};

pub fn tee_time_get_sys_time() -> axhal::time::TimeValue {
    wall_time()
}
fn tee_time_get_ree_time() -> axhal::time::TimeValue {
    wall_time()
}

pub fn sys_tee_scn_get_time(cat: u64, teetime: &mut TeeTime) -> TeeResult {
    // Get current session context
    let uuid = with_tee_session_ctx(|ctx| Ok(ctx.clnt_id.uuid))?;

    // Initialize time variable
    let mut time_result: TeeResult<TeeTime> = Err(TEE_ERROR_BAD_PARAMETERS);

    // Get time based on category
    match cat {
        0 => {
            // UTEE_TIME_CAT_SYSTEM
            let sys_time = tee_time_get_sys_time();
            time_result = Ok(TeeTime {
                seconds: sys_time.as_secs() as u32,
                millis: sys_time.subsec_millis(),
            });
        }
        1 => {
            // UTEE_TIME_CAT_TA_PERSISTENT
            time_result = tee_time_get_ta_time(&uuid);
        }
        2 => {
            // UTEE_TIME_CAT_REE
            let ree_time = tee_time_get_ree_time();
            time_result = Ok(TeeTime {
                seconds: ree_time.as_secs() as u32,
                millis: ree_time.subsec_millis(),
            });
        }
        _ => {
            return Err(TEE_ERROR_BAD_PARAMETERS);
        }
    }

    // Handle time retrieval result
    match time_result {
        Ok(time_value) => {
            // Use copy_to_user_struct to copy time to user space
            copy_to_user_struct(teetime, &time_value)
        }
        Err(e) if e == TEE_ERROR_OVERFLOW => {
            // Copy data even on overflow
            let time_value = tee_time_get_sys_time();
            let fallback_time = TeeTime {
                seconds: time_value.as_secs() as u32,
                millis: time_value.subsec_millis(),
            };
            copy_to_user_struct(teetime, &fallback_time)?;
            Err(TEE_ERROR_OVERFLOW)
        }
        Err(e) => Err(e),
    }
}

pub fn sys_tee_scn_set_ta_time(mytime: &TeeTime) -> TeeResult {
    // Copy time data from user space to kernel space
    let mut t: TeeTime = TeeTime {
        seconds: 0,
        millis: 0,
    };
    copy_from_user_struct(&mut t, mytime)?;

    // Get current session context and set TA time
    with_tee_session_ctx_mut(|ctx| tee_time_set_ta_time(&ctx.clnt_id.uuid, &t))?;

    Ok(())
}

// TA time offset structure
struct TeeTaTimeOffs {
    uuid: TEE_UUID,
    offs: TeeTime,
    positive: bool,
}

// Global time offset storage - using spin::Mutex for thread safety
use spin::Mutex;
static TEE_TIME_OFFS: Mutex<Option<Vec<TeeTaTimeOffs>>> = Mutex::new(None);

// Helper function: compare UUIDs
fn uuid_equal(uuid1: &TEE_UUID, uuid2: &TEE_UUID) -> bool {
    uuid1.timeLow == uuid2.timeLow
        && uuid1.timeMid == uuid2.timeMid
        && uuid1.timeHiAndVersion == uuid2.timeHiAndVersion
        && uuid1.clockSeqAndNode == uuid2.clockSeqAndNode
}

// Get TA time offset
fn tee_time_ta_get_offs(uuid: &TEE_UUID) -> TeeResult<(TeeTime, bool)> {
    let offs_guard = TEE_TIME_OFFS.lock();

    if let Some(ref offsets) = *offs_guard {
        for entry in offsets {
            if uuid_equal(uuid, &entry.uuid) {
                return Ok((
                    TeeTime {
                        seconds: entry.offs.seconds,
                        millis: entry.offs.millis,
                    },
                    entry.positive,
                ));
            }
        }
    }

    Err(TEE_ERROR_TIME_NOT_SET)
}

// Set TA time offset
fn tee_time_ta_set_offs(uuid: &TEE_UUID, offs: &TeeTime, positive: bool) -> TeeResult {
    let mut offs_guard = TEE_TIME_OFFS.lock();

    if let Some(ref mut offsets) = *offs_guard {
        // Find existing entry and update
        for entry in offsets.iter_mut() {
            if uuid_equal(uuid, &entry.uuid) {
                entry.offs.seconds = offs.seconds;
                entry.offs.millis = offs.millis;
                entry.positive = positive;
                return Ok(());
            }
        }

        // Add new entry
        offsets.push(TeeTaTimeOffs {
            uuid: *uuid,
            offs: TeeTime {
                seconds: offs.seconds,
                millis: offs.millis,
            },
            positive,
        });
    } else {
        // Initialize vector and add first entry
        let mut new_offsets = Vec::new();
        new_offsets.push(TeeTaTimeOffs {
            uuid: *uuid,
            offs: TeeTime {
                seconds: offs.seconds,
                millis: offs.millis,
            },
            positive,
        });
        *offs_guard = Some(new_offsets);
    }

    Ok(())
}

// Get TA time
pub fn tee_time_get_ta_time(uuid: &TEE_UUID) -> TeeResult<TeeTime> {
    let (offs, positive) = tee_time_ta_get_offs(uuid)?;
    let t = tee_time_get_sys_time();

    // Execute time calculation
    let t2 = if positive {
        // Check overflow and execute addition
        let seconds_sum = t.as_secs() + offs.seconds as u64;
        let millis_sum = t.subsec_millis() + offs.millis;

        // Handle millisecond carry
        let (final_seconds, final_millis) = if millis_sum >= 1000 {
            (seconds_sum + (millis_sum / 1000) as u64, millis_sum % 1000)
        } else {
            (seconds_sum, millis_sum)
        };

        // Detect overflow
        if final_seconds < t.as_secs() {
            return Err(TEE_ERROR_OVERFLOW);
        }

        TimeValue::new(final_seconds, final_millis as u32 * 1_000_000)
    } else {
        // Execute subtraction
        let mut seconds_diff = t.as_secs().saturating_sub(offs.seconds as u64);
        let mut millis_diff = t.subsec_millis();

        if millis_diff < offs.millis {
            if seconds_diff > 0 {
                seconds_diff -= 1;
                millis_diff += 1000 - offs.millis;
            } else {
                millis_diff = 0; // Prevent underflow
            }
        } else {
            millis_diff -= offs.millis;
        }

        TimeValue::new(seconds_diff, millis_diff as u32 * 1_000_000)
    };

    Ok(TeeTime {
        seconds: t2.as_secs() as u32,
        millis: t2.subsec_millis(),
    })
}

// Set TA time
pub fn tee_time_set_ta_time(uuid: &TEE_UUID, time: &TeeTime) -> TeeResult {
    // Check if time is normalized
    if time.millis >= 1000 {
        return Err(TEE_ERROR_BAD_PARAMETERS);
    }

    let t = tee_time_get_sys_time();
    let time_value = TimeValue::new(time.seconds as u64, time.millis as u32 * 1_000_000);

    if t.as_secs() < time_value.as_secs()
        || (t.as_secs() == time_value.as_secs() && t.subsec_millis() < time_value.subsec_millis())
    {
        // Calculate positive offset
        let seconds_diff = time_value.as_secs() - t.as_secs();
        let millis_diff = if time_value.subsec_millis() >= t.subsec_millis() {
            time_value.subsec_millis() - t.subsec_millis()
        } else {
            (1000 + time_value.subsec_millis()) - t.subsec_millis()
        };

        let offs = TeeTime {
            seconds: seconds_diff as u32,
            millis: millis_diff,
        };

        tee_time_ta_set_offs(uuid, &offs, true)
    } else {
        // Calculate negative offset
        let seconds_diff = t.as_secs() - time_value.as_secs();
        let millis_diff = if t.subsec_millis() >= time_value.subsec_millis() {
            t.subsec_millis() - time_value.subsec_millis()
        } else {
            (1000 + t.subsec_millis()) - time_value.subsec_millis()
        };

        let offs = TeeTime {
            seconds: seconds_diff as u32,
            millis: millis_diff,
        };

        tee_time_ta_set_offs(uuid, &offs, false)
    }
}

// Busy wait function
pub fn tee_time_busy_wait(milliseconds_delay: u32) -> TeeResult {
    let start_time = tee_time_get_sys_time();
    let delay_seconds = milliseconds_delay / 1000;
    let delay_millis = milliseconds_delay % 1000;

    let delay_duration = TimeValue::new(delay_seconds as u64, delay_millis as u32 * 1_000_000);

    let end_time = TimeValue::from_nanos(
        (start_time.as_nanos() + delay_duration.as_nanos())
            .try_into()
            .unwrap_or(u64::MAX),
    );

    loop {
        let current_time = tee_time_get_sys_time();
        if current_time.as_nanos() >= end_time.as_nanos() {
            break Ok(());
        }
        // Can add brief CPU yield here to avoid excessive CPU usage
        core::hint::spin_loop();
    }
}

pub fn sys_tee_scn_wait(milliseconds_delay: u32) -> TeeResult {
    tee_time_busy_wait(milliseconds_delay)
}
