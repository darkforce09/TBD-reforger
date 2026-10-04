//! The `view!` attribute guard's own calibration: it refuses the shapes that end a tag early or
//! leave a value unbraced, and accepts braced values, literals, paths and closures.

use super::view_attribute_findings;

/// The pager that leaked `= page_count on:click=…` as the "Older" button's text, laid out two
/// ways: the verdict does not depend on line breaks.
#[test]
fn view_attributes_guard_refuses_a_comparison_that_closes_the_tag() {
    let stacked = "fn pager() -> impl IntoView {
        view! {
            <button
                type=\"button\"
                disabled=page >= page_count
                on:click=move |_| state.page.set((page + 1).min(page_count))
            >
                \"Older\"
            </button>
        }
    }";
    let inline =
        "view! { <button disabled=page >= page_count on:click=move |_| next()>\"Older\"</button> }";
    for sample in [stacked, inline] {
        let findings = view_attribute_findings(sample);
        assert_eq!(findings.len(), 1, "{findings:?}");
        assert!(
            findings[0].contains("<button> closes inside"),
            "{findings:?}"
        );
    }
    let greater = view_attribute_findings("view! { <input prop:disabled=move || count > 3 /> }");
    assert!(
        greater
            .iter()
            .any(|finding| finding.contains("in the middle of an expression")),
        "{greater:?}"
    );
}

#[test]
fn view_attributes_guard_refuses_unbraced_comparisons_and_calls() {
    for (sample, attribute) in [
        (
            "view! { <button disabled=page <= 1>\"Newer\"</button> }",
            "disabled",
        ),
        ("view! { <b hidden=count == 0 /> }", "hidden"),
        ("view! { <b hidden=ready && open /> }", "hidden"),
        ("view! { <input value=id.to_string() /> }", "value"),
        ("view! { <span class=badge(level)>\"x\"</span> }", "class"),
        (
            "view! { <Row title=view! { {name} }.into_any() /> }",
            "title",
        ),
        ("view! { <b hidden=move || count < 3 /> }", "hidden"),
    ] {
        let findings = view_attribute_findings(sample);
        assert!(
            findings
                .iter()
                .any(|finding| finding.contains(&format!(" {attribute}=…>"))),
            "{sample}: {findings:?}"
        );
    }
}

#[test]
fn view_attributes_guard_accepts_braced_values_literals_paths_and_closures() {
    let sample = r##"
        // A comment may read disabled=page >= page_count without tripping the guard.
        fn pager() -> impl IntoView {
            let note = "a >= b -> c";
            view! {
                <nav class=PAGER_CLASS aria-label="a > b" data-note=r#"x >= y"#>
                    <button
                        disabled={page >= page_count}
                        on:click=move |_| state.page.set((page + 1).min(page_count))
                    >
                        "Older >"
                    </button>
                    <Select value={Signal::derive(move || query.get().to_string())} open=state.open />
                    <span class:active=move || selected.get() == id>{format!("{page} > {count}")}</span>
                    <Dialog open=open title=Titles::DELETE>{if c == '>' { "gt" } else { "" }}</Dialog>
                    <MaterialIcon name="add" class="text-sm" />
                </nav>
            }
        }
    "##;
    assert_eq!(view_attribute_findings(sample), Vec::<String>::new());
}
