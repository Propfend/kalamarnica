pub mod cmd;
pub mod context;
pub mod gh_cli;
pub mod repo_root;
pub mod storage;
pub mod transport;

#[cfg(test)]
pub mod test_utils {
    use std::sync::LazyLock;
    use std::sync::Mutex;

    pub static CWD_MUTEX: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
}
