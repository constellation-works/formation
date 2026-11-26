//! Formation executable.

fn main() -> std::process::ExitCode {
    formation::run(formation_core::Env::from_process())
}
