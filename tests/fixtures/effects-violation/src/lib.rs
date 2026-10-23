//! Breaks every std-API ban in `crates/libpachinko/clippy.toml` exactly once, so
//! `scripts/check-effects.sh` can fail unless each entry fires once here. Nothing
//! here runs: clippy only has to resolve it. Most breaks are full paths; a few use
//! the forms a path-matching tool would miss (an alias, `use std as s`, a glob, a
//! macro body, a function value, a method on a parameter), each still the only break
//! of its entry. One function per break, so an `exit` cannot make the rest
//! unreachable. Only the lints that would bury the findings are allowed; the
//! `disallowed_*` lints must stay at their default, warn.
#![allow(dead_code, unused_must_use, deprecated)]

use std::path::Path;
use std::time::{Instant, SystemTime};

// The clock. `elapsed` is broken on a value passed in, so `now` fires only once.
fn system_time_now() { SystemTime::now(); }
fn system_time_elapsed(t: SystemTime) { t.elapsed(); }
fn instant_now() { let clock: fn() -> Instant = Instant::now; clock(); }
fn instant_elapsed(i: Instant) { i.elapsed(); }

// Threads.
fn thread_spawn() { std::thread::spawn(|| ()); }
fn thread_scope() { std::thread::scope(|_| ()); }
fn thread_sleep(d: std::time::Duration) { std::thread::sleep(d); }
fn thread_sleep_ms() { std::thread::sleep_ms(1); }
fn thread_builder(_: std::thread::Builder) {}

// The filesystem: std::fs's functions, then its types.
fn fs_canonicalize() { std::fs::canonicalize("a"); }
fn fs_copy() { std::fs::copy("a", "b"); }
fn fs_create_dir() { std::fs::create_dir("a"); }
fn fs_create_dir_all() { std::fs::create_dir_all("a"); }
fn fs_exists() { std::fs::exists("a"); }
fn fs_hard_link() { std::fs::hard_link("a", "b"); }
fn fs_metadata() { std::fs::metadata("a"); }
fn fs_read() { use std::fs as f; f::read("a"); }
fn fs_read_dir() { std::fs::read_dir("a"); }
fn fs_read_link() { std::fs::read_link("a"); }
fn fs_read_to_string() { std::fs::read_to_string("a"); }
fn fs_remove_dir() { std::fs::remove_dir("a"); }
fn fs_remove_dir_all() { std::fs::remove_dir_all("a"); }
fn fs_remove_file() { std::fs::remove_file("a"); }
fn fs_rename() { std::fs::rename("a", "b"); }
fn fs_set_permissions(p: &std::fs::Permissions) { std::fs::set_permissions("a", p.clone()); }
fn fs_soft_link() { std::fs::soft_link("a", "b"); }
fn fs_symlink_metadata() { std::fs::symlink_metadata("a"); }
fn fs_write() { use std as s; s::fs::write("a", b""); }
fn fs_dir_builder(_: std::fs::DirBuilder) {}
fn fs_dir_entry(_: std::fs::DirEntry) {}
fn fs_file() { std::fs::File::open("a"); }
fn fs_file_times(_: std::fs::FileTimes) {}
fn fs_file_type(_: std::fs::FileType) {}
fn fs_metadata_type(_: std::fs::Metadata) {}
fn fs_open_options(_: std::fs::OpenOptions) {}
fn fs_read_dir_type(_: std::fs::ReadDir) {}
fn fs_try_lock_error(_: std::fs::TryLockError) {}

// The filesystem outside std::fs.
fn path_canonicalize(p: &Path) { p.canonicalize(); }
fn path_exists(p: &Path) { p.exists(); }
fn path_is_dir(p: &Path) { p.is_dir(); }
fn path_is_file(p: &std::path::PathBuf) { p.is_file(); }
fn path_is_symlink(p: &Path) { p.is_symlink(); }
fn path_metadata(p: &Path) { p.metadata(); }
fn path_read_dir(p: &Path) { p.read_dir(); }
fn path_read_link(p: &Path) { p.read_link(); }
fn path_symlink_metadata(p: &Path) { p.symlink_metadata(); }
fn path_try_exists(p: &Path) { p.try_exists(); }
fn unix_chown() { std::os::unix::fs::chown("a", None, None); }
fn unix_chroot() { std::os::unix::fs::chroot("a"); }
fn unix_fchown() { std::os::unix::fs::fchown(std::io::stdin(), None, None); }
fn unix_lchown() { std::os::unix::fs::lchown("a", None, None); }
fn unix_symlink() { std::os::unix::fs::symlink("a", "b"); }

// The environment.
macro_rules! args { () => { std::env::args() }; }
fn env_args() { args!(); }
fn env_args_os() { std::env::args_os(); }
fn env_current_dir() { std::env::current_dir(); }
fn env_current_exe() { std::env::current_exe(); }
fn env_home_dir() { std::env::home_dir(); }
fn env_join_paths() { std::env::join_paths(["a"]); }
fn env_remove_var() { unsafe { std::env::remove_var("A") } }
fn env_set_current_dir() { std::env::set_current_dir("a"); }
fn env_set_var() { unsafe { std::env::set_var("A", "b") } }
fn env_split_paths() { std::env::split_paths("a"); }
fn env_temp_dir() { std::env::temp_dir(); }
fn env_var() { use std::env::*; var("A"); }
fn env_var_os() { std::env::var_os("A"); }
fn env_vars() { std::env::vars(); }
fn env_vars_os() { std::env::vars_os(); }
fn env_absolute() { std::path::absolute("a"); }
fn env_args_type(_: std::env::Args) {}
fn env_args_os_type(_: std::env::ArgsOs) {}
fn env_join_paths_error(_: std::env::JoinPathsError) {}
fn env_split_paths_type(_: std::env::SplitPaths<'_>) {}
fn env_var_error(_: std::env::VarError) {}
fn env_vars_type(_: std::env::Vars) {}
fn env_vars_os_type(_: std::env::VarsOs) {}

// The network. The ToSocketAddrs bound breaks the type ban, the call the method ban.
fn net_to_socket_addrs<A: std::net::ToSocketAddrs>(a: &A) { a.to_socket_addrs(); }
fn net_addr_parse_error(_: std::net::AddrParseError) {}
fn net_incoming(_: std::net::Incoming<'_>) {}
fn net_ip_addr(_: std::net::IpAddr) {}
fn net_ipv4_addr(_: std::net::Ipv4Addr) {}
fn net_ipv6_addr(_: std::net::Ipv6Addr) {}
fn net_shutdown(_: std::net::Shutdown) {}
fn net_socket_addr(_: std::net::SocketAddr) {}
fn net_socket_addr_v4(_: std::net::SocketAddrV4) {}
fn net_socket_addr_v6(_: std::net::SocketAddrV6) {}
fn net_tcp_listener() { std::net::TcpListener::bind("a"); }
fn net_tcp_stream() { std::net::TcpStream::connect("a"); }
fn net_udp_socket() { std::net::UdpSocket::bind("a"); }
fn unix_datagram() { std::os::unix::net::UnixDatagram::unbound(); }
fn unix_listener() { std::os::unix::net::UnixListener::bind("a"); }
fn unix_stream() { std::os::unix::net::UnixStream::connect("a"); }

// Processes.
fn process_abort() { std::process::abort(); }
fn process_exit() { std::process::exit(0); }
fn process_id() { std::process::id(); }
fn process_child(_: std::process::Child) {}
fn process_child_stderr(_: std::process::ChildStderr) {}
fn process_child_stdin(_: std::process::ChildStdin) {}
fn process_child_stdout(_: std::process::ChildStdout) {}
fn process_command() { std::process::Command::new("a"); }
fn process_command_args(_: std::process::CommandArgs<'_>) {}
fn process_command_envs(_: std::process::CommandEnvs<'_>) {}
fn process_exit_code() { let _ = std::process::ExitCode::SUCCESS; }
fn process_exit_status(_: std::process::ExitStatus) {}
fn process_output(_: std::process::Output) {}
fn process_stdio(_: std::process::Stdio) {}
fn process_termination<T: std::process::Termination>(_: T) {}
