pub(crate) enum WavePlanEvent {
    Select(usize),
    Compare(usize),
    CopyText(String),
    ToggleWave0,
}
