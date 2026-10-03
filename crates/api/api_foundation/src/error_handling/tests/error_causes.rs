//! Unit coverage for [`message_with_causes`]: a lone error renders as its own message, and a chain
//! renders outermost first with every cause after it.

use super::*;

/// An error with a message and an optional cause, so a chain of any depth can be built.
#[derive(Debug)]
struct Layer {
    message: &'static str,
    cause: Option<Box<Layer>>,
}

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.message)
    }
}

impl Error for Layer {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        self.cause
            .as_deref()
            .map(|cause| cause as &(dyn Error + 'static))
    }
}

#[test]
fn an_error_without_a_source_renders_as_its_own_message() {
    let lone = Layer {
        message: "discord: empty access token",
        cause: None,
    };
    assert_eq!(message_with_causes(&lone), "discord: empty access token");
}

#[test]
fn a_chain_renders_outermost_first_with_every_cause() {
    let chain = Layer {
        message: "export source directory is unavailable",
        cause: Some(Box::new(Layer {
            message: "permission denied",
            cause: Some(Box::new(Layer {
                message: "os error 13",
                cause: None,
            })),
        })),
    };
    assert_eq!(
        message_with_causes(&chain),
        "export source directory is unavailable: permission denied: os error 13"
    );
}
