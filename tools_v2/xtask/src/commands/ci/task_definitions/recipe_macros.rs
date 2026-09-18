/// An echoed recipe line. The map lane stays a subprocess on purpose: `map` is a `tbd-tools`
/// binary, and reaching into another crate's clap wiring to save a fork would be drift.
macro_rules! sh {
    ($line:expr) => {
        Step::Cmd {
            line: $line,
            silent: false,
        }
    };
}

/// `cargo run -q -p xtask -- <cmd>` in the Makefile; an in-process call here.
macro_rules! xt {
    ($echo:expr, $silent:expr, $run:expr) => {
        Step::Xtask {
            echo: $echo,
            silent: $silent,
            run: $run,
        }
    };
}
