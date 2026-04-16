#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub enum RestrictionRenderMode {
    #[default]
    Property,
    ValuesFrom,
    ExistingProperty,
}

impl RestrictionRenderMode {
    pub const fn priority(self) -> u8 {
        match self {
            Self::Property => 0,
            Self::ValuesFrom => 1,
            Self::ExistingProperty => 2,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RestrictionState {
    pub on_property: Option<usize>,
    pub filler: Option<usize>,
    pub cardinality: Option<(String, Option<String>)>,
    pub self_restriction: bool,
    pub requires_filler: bool,
    pub render_mode: RestrictionRenderMode,
}
