use std::fs::File;
use std::io::{Result, Write};
use std::path::PathBuf;
use sysnum::gen_syscalls;

fn gen_syscall_nrs(dest: PathBuf) -> Result<()> {
    let mut f = File::create(dest)?;
    writeln!(f, "// AUTOMATICALLY GENERATED BY reverie/syscalls/build.rs. DO NOT EDIT.\n")?;
    writeln!(f, "pub use self::SyscallNo::*;")?;
    writeln!(f, "use core::fmt;")?;

    writeln!(
        f,
        "#[allow(non_snake_case, non_camel_case_types, non_upper_case_globals)]\n"
    )?;
    writeln!(f, "#[derive(PartialEq, Eq, Clone, Copy)]")?;
    writeln!(f, "pub enum SyscallNo {{")?;
    let syscalls = gen_syscalls().unwrap();
    for (name, nr) in &syscalls {
        writeln!(
            f,
            "    SYS{} = {},",
            name.chars().skip(4).collect::<String>(),
            nr
        )?;
    }
    writeln!(f, "}}")?;

    writeln!(f, "static SYSCALL_NAMES: [&str; {}] = [", syscalls.len())?;
    for (name, _) in &syscalls {
        writeln!(
            f,
            "    \"{}\",",
            name.chars().skip(5).collect::<String>().as_str()
        )?;
    }
    writeln!(f, "];")?;

    writeln!(f, "impl fmt::Debug for SyscallNo {{")?;
    writeln!(f, "    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {{")?;
    writeln!(
        f,
        "        write!(f, \"{{}}\", SYSCALL_NAMES[self.clone() as usize])"
    )?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;

    writeln!(f, "static SYSCALL_IDS: [SyscallNo; {}] = [", syscalls.len())?;
    for (name, _) in &syscalls {
        writeln!(f, "    SYS{},", name.chars().skip(4).collect::<String>())?;
    }
    writeln!(f, "];")?;

    writeln!(f, "impl From<i32> for SyscallNo {{")?;
    writeln!(f, "    fn from(item: i32) -> Self {{")?;
    writeln!(f, "        if item as usize > SYSCALL_IDS.len() {{")?;
    writeln!(f, "            panic!(\"invalid syscall: {{}}\", item)")?;
    writeln!(f, "        }} else {{")?;
    writeln!(f, "            SYSCALL_IDS[item as usize]")?;
    writeln!(f, "        }}")?;
    writeln!(f, "    }}")?;
    writeln!(f, "}}")?;

    Ok(())
}

fn main() {
    gen_syscall_nrs(PathBuf::from("src").join("nr.rs")).unwrap();
}
