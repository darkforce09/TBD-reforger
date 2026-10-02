// Code generated from JSON Schema using `cargo xtask schema codegen` (typify). DO NOT EDIT.
// Source: contracts/definitions/fleet-command.schema.json — regenerate with: cargo xtask ci schema-codegen

///load_mission and restart_with_mission are issued only by mission deployments (POST /servers/:id/deployments); the operator command route refuses them. console_command sends one administrator line (ConsoleCommandArguments) to the server's RCON console through the host agent; like start, stop, restart and the deployment actions it changes the server process, so at most one of them runs per server at a time, and nothing repeats it after an unknown outcome.
#[derive(
    ::serde::Deserialize,
    ::serde::Serialize,
    Clone,
    Copy,
    Debug,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
)]
pub enum FleetAction {
    #[serde(rename = "start")]
    Start,
    #[serde(rename = "stop")]
    Stop,
    #[serde(rename = "restart")]
    Restart,
    #[serde(rename = "list_players")]
    ListPlayers,
    #[serde(rename = "broadcast")]
    Broadcast,
    #[serde(rename = "kick")]
    Kick,
    #[serde(rename = "load_mission")]
    LoadMission,
    #[serde(rename = "restart_with_mission")]
    RestartWithMission,
    #[serde(rename = "console_command")]
    ConsoleCommand,
}
impl ::std::fmt::Display for FleetAction {
    fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
        match *self {
            Self::Start => f.write_str("start"),
            Self::Stop => f.write_str("stop"),
            Self::Restart => f.write_str("restart"),
            Self::ListPlayers => f.write_str("list_players"),
            Self::Broadcast => f.write_str("broadcast"),
            Self::Kick => f.write_str("kick"),
            Self::LoadMission => f.write_str("load_mission"),
            Self::RestartWithMission => f.write_str("restart_with_mission"),
            Self::ConsoleCommand => f.write_str("console_command"),
        }
    }
}
impl ::std::str::FromStr for FleetAction {
    type Err = super::error::ConversionError;
    fn from_str(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        match value {
            "start" => Ok(Self::Start),
            "stop" => Ok(Self::Stop),
            "restart" => Ok(Self::Restart),
            "list_players" => Ok(Self::ListPlayers),
            "broadcast" => Ok(Self::Broadcast),
            "kick" => Ok(Self::Kick),
            "load_mission" => Ok(Self::LoadMission),
            "restart_with_mission" => Ok(Self::RestartWithMission),
            "console_command" => Ok(Self::ConsoleCommand),
            _ => Err("invalid value".into()),
        }
    }
}
impl ::std::convert::TryFrom<&str> for FleetAction {
    type Error = super::error::ConversionError;
    fn try_from(value: &str) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<&::std::string::String> for FleetAction {
    type Error = super::error::ConversionError;
    fn try_from(
        value: &::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
impl ::std::convert::TryFrom<::std::string::String> for FleetAction {
    type Error = super::error::ConversionError;
    fn try_from(
        value: ::std::string::String,
    ) -> ::std::result::Result<Self, super::error::ConversionError> {
        value.parse()
    }
}
