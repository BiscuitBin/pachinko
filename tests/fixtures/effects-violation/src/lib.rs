//! Breaks every std-API ban in `crates/libpachinko/clippy.toml` exactly once, so
//! `scripts/check-effects.sh` can fail unless each entry fires once here. Nothing
//! here runs: clippy only has to resolve it. Most breaks are full paths; a few take
//! indirect forms (an alias, `use std as s`, a glob, a macro body, a function value,
//! a method on a parameter), each still the only break of its entry, so the bans are
//! shown to resolve names rather than match text. One function per break, so an
//! `exit` cannot make the rest unreachable; `set_var` and `remove_var` are taken as
//! values, so nothing here needs `unsafe`.
//!
//! Each break ends in a `// ban:` tag naming the entries it trips. The tags are the
//! inventory the script holds the clippy.toml entries to, so deleting a ban there
//! fails the gate until its break and tag are deleted here too. Only the lints that
//! would bury the findings are allowed; the `disallowed_*` lints stay at their
//! default, warn. The script reads each break and its tag from one line that starts
//! `fn `, so this crate's rustfmt.toml turns formatting off.
#![forbid(unsafe_code)]
#![allow(dead_code, unused_must_use, deprecated)]

use std::path::Path;
use std::time::{Instant, SystemTime};

// The clock. `elapsed` is broken on a value passed in, so `now` fires only once.
fn system_time_now() { SystemTime::now(); } // ban: std::time::SystemTime::now
fn system_time_elapsed(t: SystemTime) { t.elapsed(); } // ban: std::time::SystemTime::elapsed
fn instant_now() { let clock: fn() -> Instant = Instant::now; clock(); } // ban: std::time::Instant::now
fn instant_elapsed(i: Instant) { i.elapsed(); } // ban: std::time::Instant::elapsed

// Threads. `spawn` is broken on a scope passed in, and the timed waits, which
// read the clock, sit beside `park_timeout`.
fn thread_spawn() { std::thread::spawn(|| ()); } // ban: std::thread::spawn
fn thread_scope() { std::thread::scope(|_| ()); } // ban: std::thread::scope
fn thread_scope_spawn<'s>(s: &'s std::thread::Scope<'s, '_>) { s.spawn(|| ()); } // ban: std::thread::Scope::spawn
fn thread_sleep(d: std::time::Duration) { std::thread::sleep(d); } // ban: std::thread::sleep
fn thread_sleep_ms() { std::thread::sleep_ms(1); } // ban: std::thread::sleep_ms
fn thread_builder(_: std::thread::Builder) {} // ban: std::thread::Builder
fn thread_park_timeout(d: std::time::Duration) { std::thread::park_timeout(d); } // ban: std::thread::park_timeout
fn thread_park_timeout_ms() { std::thread::park_timeout_ms(1); } // ban: std::thread::park_timeout_ms
fn condvar_wait_timeout(c: &std::sync::Condvar, g: std::sync::MutexGuard<'_, ()>, d: std::time::Duration) { c.wait_timeout(g, d); } // ban: std::sync::Condvar::wait_timeout
fn condvar_wait_timeout_ms(c: &std::sync::Condvar, g: std::sync::MutexGuard<'_, ()>) { c.wait_timeout_ms(g, 1); } // ban: std::sync::Condvar::wait_timeout_ms
fn condvar_wait_timeout_while(c: &std::sync::Condvar, g: std::sync::MutexGuard<'_, ()>, d: std::time::Duration) { c.wait_timeout_while(g, d, |()| false); } // ban: std::sync::Condvar::wait_timeout_while
fn mpsc_recv_timeout(r: &std::sync::mpsc::Receiver<()>, d: std::time::Duration) { r.recv_timeout(d); } // ban: std::sync::mpsc::Receiver::recv_timeout
fn thread_available_parallelism() { std::thread::available_parallelism(); } // ban: std::thread::available_parallelism

// Randomness: std's own hasher seeds from the operating system.
fn hash_random_state(_: std::hash::RandomState) {} // ban: std::hash::RandomState

// The filesystem: std::fs's functions, then its types.
fn fs_canonicalize() { std::fs::canonicalize("a"); } // ban: std::fs::canonicalize
fn fs_copy() { std::fs::copy("a", "b"); } // ban: std::fs::copy
fn fs_create_dir() { std::fs::create_dir("a"); } // ban: std::fs::create_dir
fn fs_create_dir_all() { std::fs::create_dir_all("a"); } // ban: std::fs::create_dir_all
fn fs_exists() { std::fs::exists("a"); } // ban: std::fs::exists
fn fs_hard_link() { std::fs::hard_link("a", "b"); } // ban: std::fs::hard_link
fn fs_metadata() { std::fs::metadata("a"); } // ban: std::fs::metadata
fn fs_read() { use std::fs as f; f::read("a"); } // ban: std::fs::read
fn fs_read_dir() { std::fs::read_dir("a"); } // ban: std::fs::read_dir
fn fs_read_link() { std::fs::read_link("a"); } // ban: std::fs::read_link
fn fs_read_to_string() { std::fs::read_to_string("a"); } // ban: std::fs::read_to_string
fn fs_remove_dir() { std::fs::remove_dir("a"); } // ban: std::fs::remove_dir
fn fs_remove_dir_all() { std::fs::remove_dir_all("a"); } // ban: std::fs::remove_dir_all
fn fs_remove_file() { std::fs::remove_file("a"); } // ban: std::fs::remove_file
fn fs_rename() { std::fs::rename("a", "b"); } // ban: std::fs::rename
fn fs_set_permissions(p: &std::fs::Permissions) { std::fs::set_permissions("a", p.clone()); } // ban: std::fs::Permissions std::fs::set_permissions
// `set_times` takes its `FileTimes` inferred, so that type's ban does not fire twice:
// a type ban fires where the type is named, not on a value whose type is inferred.
fn fs_set_times() { std::fs::set_times("a", Default::default()); } // ban: std::fs::set_times
fn fs_set_times_nofollow() { std::fs::set_times_nofollow("a", Default::default()); } // ban: std::fs::set_times_nofollow
fn fs_soft_link() { std::fs::soft_link("a", "b"); } // ban: std::fs::soft_link
fn fs_symlink_metadata() { std::fs::symlink_metadata("a"); } // ban: std::fs::symlink_metadata
fn fs_write() { use std as s; s::fs::write("a", b""); } // ban: std::fs::write
fn fs_dir_builder(_: std::fs::DirBuilder) {} // ban: std::fs::DirBuilder
fn fs_dir_entry(_: std::fs::DirEntry) {} // ban: std::fs::DirEntry
fn fs_file() { std::fs::File::open("a"); } // ban: std::fs::File
fn fs_file_times(_: std::fs::FileTimes) {} // ban: std::fs::FileTimes
fn fs_file_type(_: std::fs::FileType) {} // ban: std::fs::FileType
fn fs_metadata_type(_: std::fs::Metadata) {} // ban: std::fs::Metadata
fn fs_open_options(_: std::fs::OpenOptions) {} // ban: std::fs::OpenOptions
fn fs_read_dir_type(_: std::fs::ReadDir) {} // ban: std::fs::ReadDir
fn fs_try_lock_error(_: std::fs::TryLockError) {} // ban: std::fs::TryLockError

// The filesystem outside std::fs.
fn path_canonicalize(p: &Path) { p.canonicalize(); } // ban: std::path::Path::canonicalize
fn path_exists(p: &Path) { p.exists(); } // ban: std::path::Path::exists
fn path_is_dir(p: &Path) { p.is_dir(); } // ban: std::path::Path::is_dir
fn path_is_file(p: &std::path::PathBuf) { p.is_file(); } // ban: std::path::Path::is_file
fn path_is_symlink(p: &Path) { p.is_symlink(); } // ban: std::path::Path::is_symlink
fn path_metadata(p: &Path) { p.metadata(); } // ban: std::path::Path::metadata
fn path_read_dir(p: &Path) { p.read_dir(); } // ban: std::path::Path::read_dir
fn path_read_link(p: &Path) { p.read_link(); } // ban: std::path::Path::read_link
fn path_symlink_metadata(p: &Path) { p.symlink_metadata(); } // ban: std::path::Path::symlink_metadata
fn path_try_exists(p: &Path) { p.try_exists(); } // ban: std::path::Path::try_exists
fn unix_chown() { std::os::unix::fs::chown("a", None, None); } // ban: std::os::unix::fs::chown
fn unix_chroot() { std::os::unix::fs::chroot("a"); } // ban: std::os::unix::fs::chroot
fn unix_fchown() { std::os::unix::fs::fchown(std::io::stdin(), None, None); } // ban: std::os::unix::fs::fchown
fn unix_lchown() { std::os::unix::fs::lchown("a", None, None); } // ban: std::os::unix::fs::lchown
fn unix_symlink() { std::os::unix::fs::symlink("a", "b"); } // ban: std::os::unix::fs::symlink

// The environment.
macro_rules! args { () => { std::env::args() }; }
fn env_args() { args!(); } // ban: std::env::args
fn env_args_os() { std::env::args_os(); } // ban: std::env::args_os
fn env_current_dir() { std::env::current_dir(); } // ban: std::env::current_dir
fn env_current_exe() { std::env::current_exe(); } // ban: std::env::current_exe
fn env_home_dir() { std::env::home_dir(); } // ban: std::env::home_dir
fn env_join_paths() { std::env::join_paths(["a"]); } // ban: std::env::join_paths
fn env_remove_var() { let _ = std::env::remove_var::<&str>; } // ban: std::env::remove_var
fn env_set_current_dir() { std::env::set_current_dir("a"); } // ban: std::env::set_current_dir
fn env_set_var() { let _ = std::env::set_var::<&str, &str>; } // ban: std::env::set_var
fn env_split_paths() { std::env::split_paths("a"); } // ban: std::env::split_paths
fn env_temp_dir() { std::env::temp_dir(); } // ban: std::env::temp_dir
fn env_var() { use std::env::*; var("A"); } // ban: std::env::var
fn env_var_os() { std::env::var_os("A"); } // ban: std::env::var_os
fn env_vars() { std::env::vars(); } // ban: std::env::vars
fn env_vars_os() { std::env::vars_os(); } // ban: std::env::vars_os
fn env_absolute() { std::path::absolute("a"); } // ban: std::path::absolute
fn env_args_type(_: std::env::Args) {} // ban: std::env::Args
fn env_args_os_type(_: std::env::ArgsOs) {} // ban: std::env::ArgsOs
fn env_join_paths_error(_: std::env::JoinPathsError) {} // ban: std::env::JoinPathsError
fn env_split_paths_type(_: std::env::SplitPaths<'_>) {} // ban: std::env::SplitPaths
fn env_var_error(_: std::env::VarError) {} // ban: std::env::VarError
fn env_vars_type(_: std::env::Vars) {} // ban: std::env::Vars
fn env_vars_os_type(_: std::env::VarsOs) {} // ban: std::env::VarsOs

// The network. The ToSocketAddrs bound breaks the type ban, the call the method ban.
fn net_to_socket_addrs<A: std::net::ToSocketAddrs>(a: &A) { a.to_socket_addrs(); } // ban: std::net::ToSocketAddrs std::net::ToSocketAddrs::to_socket_addrs
fn net_addr_parse_error(_: std::net::AddrParseError) {} // ban: std::net::AddrParseError
fn net_incoming(_: std::net::Incoming<'_>) {} // ban: std::net::Incoming
fn net_ip_addr(_: std::net::IpAddr) {} // ban: std::net::IpAddr
fn net_ipv4_addr(_: std::net::Ipv4Addr) {} // ban: std::net::Ipv4Addr
fn net_ipv6_addr(_: std::net::Ipv6Addr) {} // ban: std::net::Ipv6Addr
fn net_shutdown(_: std::net::Shutdown) {} // ban: std::net::Shutdown
fn net_socket_addr(_: std::net::SocketAddr) {} // ban: std::net::SocketAddr
fn net_socket_addr_v4(_: std::net::SocketAddrV4) {} // ban: std::net::SocketAddrV4
fn net_socket_addr_v6(_: std::net::SocketAddrV6) {} // ban: std::net::SocketAddrV6
fn net_tcp_listener() { std::net::TcpListener::bind("a"); } // ban: std::net::TcpListener
fn net_tcp_stream() { std::net::TcpStream::connect("a"); } // ban: std::net::TcpStream
fn net_udp_socket() { std::net::UdpSocket::bind("a"); } // ban: std::net::UdpSocket
fn unix_incoming(_: std::os::unix::net::Incoming<'_>) {} // ban: std::os::unix::net::Incoming
fn unix_datagram() { std::os::unix::net::UnixDatagram::unbound(); } // ban: std::os::unix::net::UnixDatagram
fn unix_listener() { std::os::unix::net::UnixListener::bind("a"); } // ban: std::os::unix::net::UnixListener
fn unix_stream() { std::os::unix::net::UnixStream::connect("a"); } // ban: std::os::unix::net::UnixStream

// Processes.
fn process_abort() { std::process::abort(); } // ban: std::process::abort
fn process_exit() { std::process::exit(0); } // ban: std::process::exit
fn process_id() { std::process::id(); } // ban: std::process::id
fn unix_parent_id() { std::os::unix::process::parent_id(); } // ban: std::os::unix::process::parent_id
fn process_child(_: std::process::Child) {} // ban: std::process::Child
fn process_child_stderr(_: std::process::ChildStderr) {} // ban: std::process::ChildStderr
fn process_child_stdin(_: std::process::ChildStdin) {} // ban: std::process::ChildStdin
fn process_child_stdout(_: std::process::ChildStdout) {} // ban: std::process::ChildStdout
fn process_command() { std::process::Command::new("a"); } // ban: std::process::Command
fn process_command_args(_: std::process::CommandArgs<'_>) {} // ban: std::process::CommandArgs
fn process_command_envs(_: std::process::CommandEnvs<'_>) {} // ban: std::process::CommandEnvs
fn process_exit_code() { let _ = std::process::ExitCode::SUCCESS; } // ban: std::process::ExitCode
fn process_exit_status(_: std::process::ExitStatus) {} // ban: std::process::ExitStatus
fn process_output(_: std::process::Output) {} // ban: std::process::Output
fn process_stdio(_: std::process::Stdio) {} // ban: std::process::Stdio
fn process_termination<T: std::process::Termination>(_: T) {} // ban: std::process::Termination
