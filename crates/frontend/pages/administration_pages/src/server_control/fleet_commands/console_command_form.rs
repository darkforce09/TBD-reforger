//! The console box: one line for the server's RCON console, requested as a fleet command that the
//! host agent carries out.
//!
//! **Role:** the "Server console" form of the command console — the line, its byte count, the
//! check the API makes, and the send control, which Enter in the field also presses.
//! **Position:** in the request controls of the fleet command section of the selected server's
//! card, under the process-control buttons; the reply shows in the followed command's panel and in
//! the history of [`super::command_history`].
//! **Signals & state:** owns the line being typed and the last failed check; sends through the
//! [`CommandConsole`].
//! **Invariants:** a line is checked as the API checks it before anything is sent, through
//! [`validated_console_line`], and the trimmed line it answers is what is sent. Nothing is sent
//! while another request is in flight, and a sent line leaves the field, so one press sends one
//! line once: the host agent transmits it once and nothing repeats it.

#[cfg(target_arch = "wasm32")]
use super::CommandConsole;
#[cfg(target_arch = "wasm32")]
use super::command_requests::{BUTTON, FIELD};
#[cfg(target_arch = "wasm32")]
use super::command_wording::validated_console_line;
#[cfg(target_arch = "wasm32")]
use frontend_api_dtos::FleetCommandRequest;
#[cfg(target_arch = "wasm32")]
use frontend_ui::MaterialIcon;
#[cfg(target_arch = "wasm32")]
use leptos::prelude::*;

/// The console box of one server's command console.
#[cfg(target_arch = "wasm32")]
pub(super) fn console_command_form(console: CommandConsole) -> impl IntoView {
    let line = RwSignal::new(String::new());
    let problem = RwSignal::new(None::<String>);
    let send = move |ev: leptos::ev::SubmitEvent| {
        ev.prevent_default();
        if console.busy.get_untracked() {
            return;
        }
        match validated_console_line(&line.get_untracked()) {
            Ok(checked) => {
                problem.set(None);
                line.set(String::new());
                console.request(FleetCommandRequest::console_command(&checked));
            }
            Err(why) => problem.set(Some(why)),
        }
    };
    view! {
        <form class="rounded-xl border border-white/10 p-3" data-testid="fleet-command-console" on:submit=send>
            <p class="mb-2 flex items-center justify-between gap-2 text-label-sm font-medium text-on-surface">
                "Server console"
                <span class="font-mono text-xs font-normal text-outline">"RCON · host agent"</span>
            </p>
            <div class="flex gap-2">
                <input aria-label="Console line" placeholder="One line for the server console, such as #players"
                    autocomplete="off" spellcheck="false"
                    prop:value=move || line.get()
                    on:input=move |ev| line.set(event_target_value(&ev))
                    class=format!("{FIELD} font-mono") />
                <button type="submit" class=BUTTON data-testid="fleet-command-console-send"
                    prop:disabled=move || console.busy.get() || line.with(|l| l.trim().is_empty())>
                    <MaterialIcon name="send" class="text-[16px]" />
                    "Send"
                </button>
            </div>
            <p class="mt-1 flex justify-between gap-3 text-xs">
                <span class="text-error-alert">{move || problem.get()}</span>
                <span class="font-mono text-outline">
                    {move || format!(
                        "{} / {} bytes",
                        line.with(|l| l.trim().len()),
                        FleetCommandRequest::CONSOLE_LINE_MAX_BYTES
                    )}
                </span>
            </p>
            <p class="mt-1 text-xs text-on-surface-variant">
                "The host agent sends the line once and never repeats it; with no reply, the outcome says it may or may not have run."
            </p>
        </form>
    }
}
