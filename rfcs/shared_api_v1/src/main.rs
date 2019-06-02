#![feature(async_await)]
#![allow(dead_code)]

use libc::pid_t;
use nix::sys::signal::Signal;
use serde::Serialize;
use std::fmt::Debug;
use std::ptr::NonNull;

/// Instrumentor configuration set at startup time.
pub struct StaticConfig {
    mode : InstrumentMode,
    init_dynconfig : DynConfig,
}

/// Dynamic configuration options that may change after each handler execution.
pub struct DynConfig {    
    heartbeat : Heartbeat,
}

/// How often to interupt the guest (fire a timer) causing an event to be 
/// created and handled by the tool.
pub enum Heartbeat {
    /// No heartbeat.  Guests will only yield when they trigger a relevant event, 
    /// not merely due to the passage of time.
    NoBeat,

    /// Future/TODO: A maximum guest compute iota, specified in units of 
    /// retired branch conditionals.  This heartbeat can be used to construct 
    /// a deterministic logical clock (DLC), but it is expensive, because current
    /// (2019) Intel hardware does not support exact interrupts on this perf counter
    /// so some single-stepping is required (see RR ATC'17 paper).
    /// 
    /// The boolean indicates whether other handled events "count" as heartbeats.
    /// If true, then the heartbeat only triggers if and when the guest exceeds
    /// its time slice before yielding with a syscall or some other event.
    ExactRBCs(u64, bool),

    /// Future/TODO: a nondeterministic heartbeat that is less expensive to implement.
    /// This can be useful for updating ones own clock, for example (i.e. in a scenario 
    /// where we do not yield on heartbeats, but do publish state).
    ApproxCyclesRBCs(u64, bool),
    /// Future/TODO: The same as above, but in units of cycles rather than RBCs.
    ApproxCycles(u64, bool),
}

pub enum InstrumentMode {
    /// Fully centralized tool execution inside a tracer process.  
    /// 
    /// This uses ptrace to handle all events, and for all guest access/modification.
    FullPtrace,

    /// Execute event handlers inside guest process when possible.  
    /// 
    /// In this default setting, local handlers communicate with the 
    /// global state object using ________, the default RPC 
    /// implementation.  Global state methods run centrally in a tracer
    /// and they read and modify (inject) the guest processes using ptrace.
    InGuestDefault,

    // TODO: in the future we may offer a mode for executing global methods 
    // in a decentralized fashion, assuming threadsafe implementations and all 
    // global state managed in shared pages.  We're setting aside this option 
    // for the near and medium term.
}

/// Events are the guest actions/state changes that the tool responds to.
///
/// These are the "upcalls" into the tool, from the guest(s).
///
#[derive(PartialEq, Debug, Eq, Hash, Clone)]
pub enum Event {
    /// An attempt to execute a syscall inside the guest.  Note, the interceptor
    /// may configured to only intercept a *subset* of syscalls, which will prune the events
    /// that appear in this form.
    Syscall(SysNo, SysArgs),

    /// A trapped instruction in the guest, other than a syscall.
    Instruction(Instr),

    /// A signal received within the guest.
    Signal(Signal),

    /// An (optional) notification that exit will happen for this thread.
    /// QUESTION: Can this be removed?
    PreExit(TID),

    /// Exit of a thread.  This is not defined as requiring that the OS has freed 
    /// resources.  Rather, the definition is that no further events or observable 
    /// side effects will be seen from this TID.
    ExitThread(TID),
    /// Same but for processes.
    ExitProc(pid_t),

    /// Future/TODO: 
    /// Timer/heartbeat events: for future use with a deterministic (DLC) implementation.
    /// The guest yields cooperatively when it finishes its logical time slice.
    /// The heartbeat carries with it the current thread time, in whatever unit was requested.
    HeartbeatYield(u64),
}

/// An Event together with information on where it came from.
pub struct FullEvent {
    e : Event,
    tid : TID,
    pid : pid_t,
}

/// Run code *inside* a guest process.
///
/// The Injector interface provides the "downcalls".
/// The injector inserts either new system calls or function calls into the guest.  
/// It does *not* create (JIT compile) new functions in the guest, rather it calls
/// existing functions. (Though it does inject new code in the case of individual syscalls.)
///
/// NOTE: there is currently no way to inject *signals* into the guest.  Rather, one must
/// inject the functional calls, and instead intercept and prevent attempts by the guest
/// to register signal handlers in the first place.
pub trait Injector {
    /// Inject a system call into the guest and register the callback.
    /// Note that the callback will be called twice in the case of a Fork.
    fn inject_syscall(_: SysNo, _: SysArgs, k: fn(_: SysCallRet) -> ()) -> ();

    /// Look up the address of a function within the guest.
    fn resolve_symbol_address(&self, _: pid_t, _: String) -> FunAddr;

    /// Run a function in the guest.
    ///
    /// TODO: ideally a tool implementing SystraceTool would be able to
    /// call its own functions within the guest without indirecting through the
    fn inject_funcall(&self, func: FunAddr, args: &[u64; 6]) -> i64;

    /// Wait for the guest to exit.
    fn wait_exit() -> ();

    // inject_signal(...) -> ...;
}

/// This can either be all registers together in memory, or an interface
/// for fetching them one at a time.
pub struct Regs {
    _rax: u64,
    // ...
}

pub trait RegsAccess {
    // TODO
}


/// Full access to the Guest includes the ability to inject,
///  as well as the ability to access the guest's state.
pub trait GuestAccess: Injector {
    fn get_regs(&self) -> Regs;
    fn get_static_config(&self) -> StaticConfig;
    fn get_dynamic_config(&self) -> DynConfig;
}

/// The interface satisfied by a complete Systrace instrumentation tool.
///
///
///
pub trait SystraceTool
where
    Self::Glob: Debug,
    Self::Proc: Debug,
    Self::Thrd: Debug,
    // Self::Tmp: Debug,
    // Processor- and Thread-local state may need to migrated:
    Self::Proc: Serialize,
    Self::Thrd: Serialize,
{
    /// Global state shared by the tool across the whole process tree being instrumented.
    type Glob;
    /// Tool state specific to the guest process.
    type Proc;
    /// Tool state specific to the guest thread.
    type Thrd;
    // Temporary state used to represent a tool's computation in the middle of
    // injecting a fresh syscall or funcall in the guest.
    // type Tmp;

    /// Initialize the tool, allocating the global state.
    ///
    /// Takes an optional buffer in which to allocate shared, global state.  
    /// This feature is unimplemeneted, but included for forward portability.
    ///
    fn init_global_state(gbuf: Option<NonNull<u8>>) -> Option<Self::Glob>;
    /// Trigger to initialize state when a process is created, including the root process.
    /// Every process includes at least one thread, so this returns a thread state as well.
    fn init_process_state(g: &Self::Glob) -> (Self::Proc, Self::Thrd);
    /// A guest process creates additional threads, which need their state initialized.
    /// This takes the thread-local state of the PARENT thread for reference.
    fn init_thread_state(g: &Self::Glob, p: &Self::Proc, parent: &Self::Thrd) -> Self::Thrd;

    /// The tool receives an event from the instrumentation.
    ///
    /// This is where all the action happens.  This async method can make asynchronous calls
    /// against either (1) the global state (remote object), or (2) the injector.
    /// In either case, it is registering a continuation to respond to the completion of an RPC
    /// in the host (coordinator) or guest respectively.
    fn handle_event<I: Injector>(e : Event, i: I, g: Self::Glob, p: &Self::Proc, t: &mut Self::Thrd)
        -> ();
        //  TODO: in the future each event handled may return a result to the instrumentor 
        // which changes its configuration: for example, subscribing or unsubscribing to 
        // categories of events.

    // Optionally override this as an optimization.  By default it will send the event 
    // to the respective
    fn handle_global_event<I: Injector>(e : Event, i: I, g: Self::Glob)
        -> ();
}

//--------------------------------------------------------------
// TODO / unfinished
//--------------------------------------------------------------

/// Thread ID
pub type TID = pid_t;

// The value returned in RAX on x86_64:
pub type SysCallRet = i64;

/// The 6 arguments of a syscall, raw untyped version.
///
/// TODO: Use a helper function to convert to a structured Syscall+Args enum.
#[derive(PartialEq, Debug, Eq, Hash, Clone)]
pub struct SysArgs {
    arg0: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
}

#[derive(PartialEq, Debug, Eq, Hash, Clone)]
pub enum Instr {
    RDTSC,    
    CPUID,
    // Future, no current way to trap:
    // RDRAND,
    // HTX instructions...
}

// TODO Grab this enum from the lib:
pub type SysNo = u64;
pub type SigNo = u64;
pub type FunAddr = u64;

//--------------------------------------------------------------
fn main() {
    println!("Hello, world!");

}
