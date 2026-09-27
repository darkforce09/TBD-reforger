//! Native metadata is rendered without substituting missing values.
use leptos::prelude::*;

pub fn metadata(text: &str) -> impl IntoView {
    let value = serde_json::from_str::<serde_json::Value>(text)
        .unwrap_or(serde_json::Value::String(text.into()));
    render(value, 0)
}
fn render(value: serde_json::Value, depth: usize) -> AnyView {
    if depth > 4 {
        return view!{<pre class="dv-value-full">{serde_json::to_string_pretty(&value).unwrap_or_default()}</pre>}.into_any();
    }
    match value {
        serde_json::Value::Object(map)=>view!{<dl class="dv-metadata">{map.into_iter().map(|(k,v)|view!{<dt>{k}</dt><dd>{render(v,depth+1)}</dd>}).collect_view()}</dl>}.into_any(),
        serde_json::Value::Array(items)=>view!{<ol class="dv-metadata-array">{items.into_iter().map(|v|view!{<li>{render(v,depth+1)}</li>}).collect_view()}</ol>}.into_any(),
        serde_json::Value::String(s)=>view!{<span>{s}</span>}.into_any(),
        v=>view!{<span>{v.to_string()}</span>}.into_any(),
    }
}
