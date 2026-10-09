//! The `acknowledgement-dropping-relay` binary: the staging relay that withholds one fleet
//! executor acknowledgement, run by the fleet staging procedure.

fn main() -> std::process::ExitCode {
    acknowledgement_dropping_relay::entrypoint()
}
