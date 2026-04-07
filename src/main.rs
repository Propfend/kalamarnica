use std::io::Write as _;

use anyhow::Result;
use clap::Parser;
use clap::Subcommand;
use env_logger::Builder;
use env_logger::Target;
use log::LevelFilter;

use kalamarnica::cmd::apply::Apply;
use kalamarnica::cmd::auth_status::AuthStatus;
use kalamarnica::cmd::bind::Bind;
use kalamarnica::cmd::create::Create;
use kalamarnica::cmd::current::Current;
use kalamarnica::cmd::delete::Delete;
use kalamarnica::cmd::handler::Handler as _;
use kalamarnica::cmd::list::List;
use kalamarnica::cmd::set_token::SetToken;
use kalamarnica::cmd::switch::Switch;
use kalamarnica::cmd::unbind::Unbind;
use kalamarnica::storage::Storage;

#[derive(Parser)]
#[command(arg_required_else_help(true), version, about, long_about = None)]
/// GitHub CLI context manager — manages multiple GitHub accounts and tokens
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// List all saved contexts
    List(List),
    /// Show the active context
    Current(Current),
    /// Create a new context
    Create(Create),
    /// Switch to a context
    Switch(Switch),
    /// Store a per-context GitHub token
    SetToken(SetToken),
    /// Delete a context
    Delete(Delete),
    /// Bind a repository to a context
    Bind(Bind),
    /// Remove repository context binding
    Unbind(Unbind),
    /// Apply the repository-bound context
    Apply(Apply),
    /// Show authentication status for all contexts
    AuthStatus(AuthStatus),
}

fn main() -> Result<()> {
    Builder::new()
        .filter_level(LevelFilter::Info)
        .format(|buffer, record| writeln!(buffer, "{}", record.args()))
        .target(Target::Stdout)
        .init();

    let storage = Storage::new()?;

    match Cli::parse().command {
        Some(Commands::List(handler)) => handler.handle(&storage),
        Some(Commands::Current(handler)) => handler.handle(&storage),
        Some(Commands::Create(handler)) => handler.handle(&storage),
        Some(Commands::Switch(handler)) => handler.handle(&storage),
        Some(Commands::SetToken(handler)) => handler.handle(&storage),
        Some(Commands::Delete(handler)) => handler.handle(&storage),
        Some(Commands::Bind(handler)) => handler.handle(&storage),
        Some(Commands::Unbind(handler)) => handler.handle(&storage),
        Some(Commands::Apply(handler)) => handler.handle(&storage),
        Some(Commands::AuthStatus(handler)) => handler.handle(&storage),
        None => Ok(()),
    }
}
