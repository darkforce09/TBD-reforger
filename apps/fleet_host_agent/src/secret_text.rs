//! Secret text: the machine credential and the RCON password.

use std::fmt;

/// A secret held in memory. `Debug` prints a placeholder and there is no `Display`, so a secret
/// cannot reach a log line through formatting. [`SecretText::expose`] is called only where the
/// secret enters its own protocol: the `Authorization` header of API requests and the RCON
/// login packet.
#[derive(Clone, PartialEq, Eq)]
pub struct SecretText(String);

impl SecretText {
    pub fn new(secret: impl Into<String>) -> Self {
        Self(secret.into())
    }

    pub fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretText {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretText(<redacted>)")
    }
}

#[cfg(test)]
#[path = "tests/secret_text.rs"]
mod tests;
