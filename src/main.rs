use std::{env, path::PathBuf, process::ExitCode};

use transform_ci::Runner;

fn main() -> ExitCode {
    let dirs = env::args().skip(1).map(PathBuf::from).collect::<Vec<PathBuf>>();
    assert!(!dirs.is_empty(), "Expected directories");
    Runner::new(dirs).run()
}
