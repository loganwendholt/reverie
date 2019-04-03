#![feature(format_args_nl)]

use syscalls::*;
use tools_helper::*;

#[allow(unused_imports)]
use std::ffi::CStr;

#[no_mangle]
pub extern "C" fn captured_syscall(
    _no: i32,
    _a0: i64,
    _a1: i64,
    _a2: i64,
    _a3: i64,
    _a4: i64,
    _a5: i64,
) -> i64 {
    let ret = unsafe { untraced_syscall(_no, _a0, _a1, _a2, _a3, _a4, _a5) };
    msg!("{:?} = {:x?}", syscalls::SyscallNo::from(_no), ret);
    ret
}
