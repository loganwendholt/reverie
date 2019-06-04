//! auxv decoder
//!
//! the auxv is passed by linux kernel to either the dynamic linker
//! or the program's entry point (static binaries)
//!
//! to get the correct values, the decoder must be called on
//! ptrace exec event. or if you're the dynamic linker :-)
//!

use std::io::Result;
use std::collections::HashMap;

use crate::task::Task;
use crate::traced_task::TracedTask;
use crate::remote::*;

const AUXV_MAX: usize = 256;

pub unsafe fn getauxval(task: &TracedTask) -> Result<HashMap<usize, u64>> {
    let mut res: HashMap<usize, u64>  = HashMap::new();
    let regs = task.getregs()?;

    let sp = RemotePtr::new(regs.rsp as *mut u64);

    let vec = task.peek_bytes(sp.cast(), AUXV_MAX * std::mem::size_of::<u64>())?;
    let auxv: Vec<u64> = std::mem::transmute(vec);
    let argc = auxv[0];
    let mut k = 2 + argc as usize;

    loop {
        if auxv[k] == 0 {
            k = 1 + k;
            break;
        }
        k = 1 + k;
    }

    loop {
        let key = auxv[k];
        if key == 0 {
            break;
        }
        let val = auxv[1+k];
        res.insert(key as usize, val);
        k = 2 + k;
    }

    Ok(res)
}
