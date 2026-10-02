//! Normalisation of the fields a write golden cannot pin: the ids, secrets and request-time
//! instants the server generates.
//!
//! **Role:** applies the committed table in [`super::normalised_fields`]. At each field the table
//! names, the live value must exist, be a string and have its kind's format (a random UUID, a
//! secret of the kind's shape, or an RFC 3339 instant stamped while the capture ran); it is then
//! replaced by the kind's fixed placeholder, which is the value the golden stores. The table is
//! also audited against the index and the goldens, so no row is dead and no placeholder sits
//! where no row names it.
//!
//! **Position:** used by the reproduction case of `tests/contract_parity_goldens.rs` before
//! [`super::json_difference::differences`], and by its index case through [`table_problems`];
//! reads the goldens through [`super::golden_index`].
//!
//! **Signals & state:** none; pure functions over the table, the goldens and one live answer.
//!
//! **Invariants:** only a `(method, path, pointer)` the table names is replaced, so every other
//! byte of the answer compares exactly; a golden stores a placeholder exactly where the table
//! names a field, and never a live secret; a failure message never prints a live secret.

use std::collections::BTreeSet;

use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use uuid::Uuid;

use super::golden_index::GoldenRow;
use super::normalised_fields::{NORMALISED_FIELDS, NormalisedField};

/// The placeholder of a server-generated id: a well-formed version 4 UUID no seed uses.
pub const UUID_PLACEHOLDER: &str = "99999999-9999-4999-9999-999999999999";
/// The placeholder of a request-time instant and of an instant derived from one.
pub const INSTANT_PLACEHOLDER: &str = "2000-01-01T00:00:00Z";
/// The placeholder of a signed access token: three base64url segments, like a JWT.
pub const ACCESS_TOKEN_PLACEHOLDER: &str = "normalised.access.token";
/// The placeholder of an opaque refresh token: 64 hexadecimal zeros.
pub const OPAQUE_TOKEN_PLACEHOLDER: &str =
    "0000000000000000000000000000000000000000000000000000000000000000";
/// The placeholder of an Arma link code.
pub const LINK_CODE_PLACEHOLDER: &str = "000000";
/// The placeholder of a machine credential secret: the UUID placeholder's digits, then zeros.
pub const MACHINE_SECRET_PLACEHOLDER: &str = "tbdm_99999999999949999999999999999999_\
    0000000000000000000000000000000000000000000000000000000000000000";

/// How far outside the capture window an instant may fall. The router stamps instants with the
/// database clock and the process clock, which read the same host clock; the database keeps
/// microseconds, so one second absorbs truncation while keeping a stale or seeded instant out.
const WINDOW_SLACK: Duration = Duration::seconds(1);

/// The span in which the capture sent its requests.
#[derive(Clone, Copy, Debug)]
pub struct CaptureWindow {
    /// Read just before the first request.
    pub opened_at: DateTime<Utc>,
    /// Read just after the last answer.
    pub closed_at: DateTime<Utc>,
}

impl CaptureWindow {
    fn contains(&self, instant: DateTime<Utc>) -> bool {
        instant >= self.opened_at - WINDOW_SLACK && instant <= self.closed_at + WINDOW_SLACK
    }
}

/// What a normalised field holds, which fixes its format check and its placeholder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NormalisedKind {
    /// An id the server generates: a lowercase, hyphenated version 4 UUID.
    ServerUuid,
    /// An RFC 3339 instant stamped while the request ran.
    RequestTime,
    /// An RFC 3339 instant this many seconds after the request ran (an expiry or a deadline).
    RequestTimePlusSeconds(i64),
    /// A signed access token: three non-empty base64url segments.
    AccessTokenJwt,
    /// An opaque refresh token: 64 lowercase hexadecimal digits.
    OpaqueTokenHex64,
    /// An Arma link code: six decimal digits.
    LinkCodeSixDigits,
    /// A machine credential secret: `tbdm_<the credential id's 32 hex digits>_<64 hex digits>`.
    MachineCredentialSecret,
}

impl NormalisedKind {
    /// The value a golden stores at a field of this kind.
    pub fn placeholder(self) -> &'static str {
        match self {
            Self::ServerUuid => UUID_PLACEHOLDER,
            Self::RequestTime | Self::RequestTimePlusSeconds(_) => INSTANT_PLACEHOLDER,
            Self::AccessTokenJwt => ACCESS_TOKEN_PLACEHOLDER,
            Self::OpaqueTokenHex64 => OPAQUE_TOKEN_PLACEHOLDER,
            Self::LinkCodeSixDigits => LINK_CODE_PLACEHOLDER,
            Self::MachineCredentialSecret => MACHINE_SECRET_PLACEHOLDER,
        }
    }

    /// Whether a live value of this kind is a credential that no message may print.
    fn is_secret(self) -> bool {
        matches!(
            self,
            Self::AccessTokenJwt
                | Self::OpaqueTokenHex64
                | Self::LinkCodeSixDigits
                | Self::MachineCredentialSecret
        )
    }

    /// `Ok` when `live` has this kind's format; otherwise why it does not.
    fn check(self, live: &str, window: &CaptureWindow) -> Result<(), String> {
        match self {
            Self::ServerUuid => random_uuid(live),
            Self::RequestTime => instant_in_window(live, Duration::zero(), window),
            Self::RequestTimePlusSeconds(seconds) => {
                instant_in_window(live, Duration::seconds(seconds), window)
            }
            Self::AccessTokenJwt => {
                let segments: Vec<&str> = live.split('.').collect();
                let base64url = |segment: &&str| {
                    !segment.is_empty()
                        && segment
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
                };
                if segments.len() == 3 && segments.iter().all(base64url) {
                    Ok(())
                } else {
                    Err("is not three base64url segments".to_string())
                }
            }
            Self::OpaqueTokenHex64 => lowercase_hex(live, 64),
            Self::LinkCodeSixDigits => {
                if live.len() == 6 && live.bytes().all(|byte| byte.is_ascii_digit()) {
                    Ok(())
                } else {
                    Err("is not six decimal digits".to_string())
                }
            }
            Self::MachineCredentialSecret => {
                let Some((id, random)) = live
                    .strip_prefix("tbdm_")
                    .and_then(|rest| rest.split_once('_'))
                else {
                    return Err("does not read tbdm_<id>_<random>".to_string());
                };
                lowercase_hex(id, 32)?;
                lowercase_hex(random, 64)?;
                let id = Uuid::parse_str(id).map_err(|_| "names no credential id".to_string())?;
                random_uuid(&id.hyphenated().to_string())
            }
        }
    }

    /// The kind as the design note's table and every failure message spell it.
    pub fn describe(self) -> String {
        match self {
            Self::ServerUuid => "server_uuid".to_string(),
            Self::RequestTime => "request_time".to_string(),
            Self::RequestTimePlusSeconds(seconds) => format!("request_time + {seconds} s"),
            Self::AccessTokenJwt => "access_token_jwt".to_string(),
            Self::OpaqueTokenHex64 => "opaque_token_hex64".to_string(),
            Self::LinkCodeSixDigits => "link_code_six_digits".to_string(),
            Self::MachineCredentialSecret => "machine_credential_secret".to_string(),
        }
    }
}

fn random_uuid(live: &str) -> Result<(), String> {
    let uuid = Uuid::parse_str(live).map_err(|_| "is not a UUID".to_string())?;
    if uuid.hyphenated().to_string() != live {
        return Err("is not a lowercase hyphenated UUID".to_string());
    }
    if uuid.get_version_num() != 4 {
        return Err(format!(
            "is a version {} UUID, not a random one",
            uuid.get_version_num()
        ));
    }
    Ok(())
}

fn lowercase_hex(live: &str, digits: usize) -> Result<(), String> {
    if live.len() == digits
        && live
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(())
    } else {
        Err(format!("is not {digits} lowercase hexadecimal digits"))
    }
}

fn instant_in_window(live: &str, after: Duration, window: &CaptureWindow) -> Result<(), String> {
    let instant = DateTime::parse_from_rfc3339(live)
        .map_err(|_| "is not an RFC 3339 instant".to_string())?
        .with_timezone(&Utc);
    if window.contains(instant - after) {
        Ok(())
    } else {
        Err(format!(
            "minus {} s is outside the capture window {} .. {}",
            after.num_seconds(),
            window.opened_at.to_rfc3339(),
            window.closed_at.to_rfc3339()
        ))
    }
}

/// The table rows that name `row`'s golden.
fn fields_of<'a>(
    method: &'a str,
    path: &'a str,
) -> impl Iterator<Item = &'static NormalisedField> + 'a {
    NORMALISED_FIELDS
        .iter()
        .filter(move |field| field.method == method && field.path == path)
}

/// The live answer with every field the table names for `row` checked and replaced by its
/// placeholder, and one problem line per field that is missing or fails its kind's check.
pub fn normalise_live(
    row: &GoldenRow,
    mut live: Value,
    window: &CaptureWindow,
) -> (Value, Vec<String>) {
    let mut problems = Vec::new();
    for field in fields_of(row.method(), &row.path) {
        let kind = field.kind;
        let at = format!("{} (normalised as {})", field.pointer, kind.describe());
        let Some(slot) = live.pointer_mut(field.pointer) else {
            problems.push(format!("{at}: missing from the live answer"));
            continue;
        };
        let verdict = match slot {
            Value::String(text) => kind.check(text, window).map_err(|why| {
                if kind.is_secret() {
                    format!("the live secret ({} characters) {why}", text.len())
                } else {
                    format!("the live value {text:?} {why}")
                }
            }),
            Value::Null => Err("the live value is null".to_string()),
            _ => Err("the live value is not a string".to_string()),
        };
        if let Err(why) = verdict {
            problems.push(format!("{at}: {why}"));
        }
        *slot = Value::String(kind.placeholder().to_string());
    }
    (live, problems)
}

/// Every disagreement between the table, the index and the goldens: a duplicate row, a row that
/// names no indexed JSON golden, a pointer its golden lacks, a golden value that is not the
/// kind's placeholder, and a placeholder a golden holds where no row names it.
pub fn table_problems(rows: &[GoldenRow]) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen = BTreeSet::new();
    for field in NORMALISED_FIELDS {
        let at = format!(
            "normalisation row {} {} {}",
            field.method, field.path, field.pointer
        );
        if !seen.insert((field.method, field.path, field.pointer)) {
            problems.push(format!("{at}: listed twice"));
        }
        let Some(row) = rows.iter().find(|row| {
            row.method() == field.method && row.path == field.path && !row.is_event_stream()
        }) else {
            problems.push(format!("{at}: names no indexed JSON golden (a dead row)"));
            continue;
        };
        match row.read_json() {
            Ok(golden) => match golden.pointer(field.pointer) {
                None => problems.push(format!("{at}: {} has no such field", row.file)),
                Some(value) if value.as_str() != Some(field.kind.placeholder()) => {
                    problems.push(format!(
                        "{at}: {} stores {value}, not the {} placeholder {:?}",
                        row.file,
                        field.kind.describe(),
                        field.kind.placeholder()
                    ));
                }
                Some(_) => {}
            },
            Err(why) => problems.push(format!("{at}: {why}")),
        }
    }
    for row in rows.iter().filter(|row| !row.is_event_stream()) {
        let Ok(golden) = row.read_json() else {
            continue;
        };
        let named: BTreeSet<&str> = fields_of(row.method(), &row.path)
            .map(|field| field.pointer)
            .collect();
        let mut placed = Vec::new();
        placeholder_pointers("", &golden, &mut placed);
        for pointer in placed
            .iter()
            .filter(|pointer| !named.contains(pointer.as_str()))
        {
            problems.push(format!(
                "{}: {pointer} holds a normalisation placeholder, but no table row names it",
                row.file
            ));
        }
    }
    problems
}

/// Every pointer under `value` whose string equals one of the kinds' placeholders.
fn placeholder_pointers(pointer: &str, value: &Value, out: &mut Vec<String>) {
    const PLACEHOLDERS: [&str; 6] = [
        UUID_PLACEHOLDER,
        INSTANT_PLACEHOLDER,
        ACCESS_TOKEN_PLACEHOLDER,
        OPAQUE_TOKEN_PLACEHOLDER,
        LINK_CODE_PLACEHOLDER,
        MACHINE_SECRET_PLACEHOLDER,
    ];
    match value {
        Value::String(text) if PLACEHOLDERS.contains(&text.as_str()) => {
            out.push(pointer.to_string());
        }
        Value::Object(map) => {
            for (key, child) in map {
                let token = key.replace('~', "~0").replace('/', "~1");
                placeholder_pointers(&format!("{pointer}/{token}"), child, out);
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                placeholder_pointers(&format!("{pointer}/{index}"), child, out);
            }
        }
        _ => {}
    }
}
