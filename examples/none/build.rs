use cc;
use std::fs::File;
use std::io::{Result, Write};
use std::path::Path;

fn main() -> Result<()> {
    cc::Build::new()
        .define("_POSIX_C_SOURCE", "20180920")
        .define("_GNU_SOURCE", "1")
        .define("USE_SAVE", "1")
        .flag("-fPIC")
        .include("../../include")
        .include("../../trampoline")
        .file("../../trampoline/trampoline.S")
        .file("../../trampoline/raw_syscall.S")
        .file("../../trampoline/remote_call.S")
        .compile("my-trampoline");

    let cwd = std::env::current_dir()?;
    let src_files: Vec<_> = vec![ std::fs::canonicalize("../../trampoline/ffi.rs").unwrap(),
                                  std::fs::canonicalize("../../src/consts.rs").unwrap()];
    let dst_dir = std::fs::canonicalize(cwd.join("src"))?;

    src_files.iter().for_each(|f| {
        let do_copy: Box<dyn Fn() -> Result<()>> = Box::new(|| {
            let p = Path::new(&f).file_name().unwrap();
            let mut src = File::open(f)?;
            let mut dst = File::create(dst_dir.join(p))?;
            let header = format!("// AUTOMATICALLY GENERATED by build.rs by copying {:?}. DO NOT EDIT.\n\n", dst_dir.join(p));
            dst.write_all(header.as_bytes())?;
            std::io::copy(&mut src, &mut dst)?;
            Ok(())
        });
        do_copy().expect("failed to copy files");
    });
    Ok(())
}
