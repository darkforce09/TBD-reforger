use ticket_engine::ScopeV2;
// ---- scope breadcrumb ----

/// Breadcrumb tier used to choose a muted semantic accent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopeLevel {
    Domain,
    Layer,
    Component,
    Surface,
}

/// A scope tier paired with its precomputed display label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BreadcrumbSeg {
    pub level: ScopeLevel,
    pub text: String,
}

/// Precomputed scope labels shared by cards and ticket details.
///
/// Component-free scopes end at the layer. When a component has no surfaces,
/// no_surface is true: details show NO_SURFACE_MARKER and cards omit that marker.
/// An inferred scope carries the estimated glyph and its explanatory tooltip.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Breadcrumb {
    pub segs: Vec<BreadcrumbSeg>,
    pub no_surface: bool,
    pub estimated: bool,
}

/// Separator glyph between breadcrumb segments.
pub const SCOPE_SEP: &str = "›";

/// Detail-panel marker for a component whose surface list is empty.
pub const NO_SURFACE_MARKER: &str = "(no surface)";

/// Glyph prefixed to owns-inferred (estimated) scope breadcrumbs.
pub const SCOPE_ESTIMATED_GLYPH: &str = "~";

/// Tooltip on the estimated-scope glyph.
pub const SCOPE_ESTIMATED_TIP: &str = "scope owns-inferred at migration";

/// The estimated[] marker predicate: was this ticket's scope owns-inferred by the
/// v2 migrator (rather than carried by v1 data)?
pub fn scope_estimated(estimated: &[String]) -> bool {
    estimated.iter().any(|e| e == "scope")
}

pub fn breadcrumb(scope: &ScopeV2, estimated: &[String]) -> Breadcrumb {
    let mut segs = vec![
        BreadcrumbSeg {
            level: ScopeLevel::Domain,
            text: scope.domain.as_str().to_owned(),
        },
        BreadcrumbSeg {
            level: ScopeLevel::Layer,
            text: scope.layer.clone(),
        },
    ];
    if let Some(component) = &scope.component {
        segs.push(BreadcrumbSeg {
            level: ScopeLevel::Component,
            text: component.clone(),
        });
    }
    if !scope.surface.is_empty() {
        segs.push(BreadcrumbSeg {
            level: ScopeLevel::Surface,
            text: scope.surface.join("+"),
        });
    }
    Breadcrumb {
        no_surface: scope.component.is_some() && scope.surface.is_empty(),
        estimated: scope_estimated(estimated),
        segs,
    }
}

impl Breadcrumb {
    /// Plain-text path — the four scope fields joined (`repo › docs`,
    /// `website › frontend › mission_creator › a+b`). The chip path paints `segs`
    /// individually; this is the canonical string form (tests + hover copy).
    pub fn label(&self) -> String {
        self.segs
            .iter()
            .map(|s| s.text.as_str())
            .collect::<Vec<_>>()
            .join(&format!(" {SCOPE_SEP} "))
    }
}
