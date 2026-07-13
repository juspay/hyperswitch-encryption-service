mod logger;
mod metrics;
pub mod observability;

/// Returns the full version string with git info.
#[cfg(feature = "vergen")]
#[macro_export]
macro_rules! version {
    () => {
        concat!(
            build_info::git_describe!(),
            "-",
            build_info::git_sha!(),
            "-",
            build_info::git_commit_timestamp!()
        )
    };
}
