// crates/reactatui/src/layout/tracks.rs — replaces both TrackSize and FlexBasis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// Opt-in intrinsic sizing. This may require a content measurement pass.
    Auto,
    Length(u16),
    Percent(u16),
    /// The one and only way to grow: a proportional share of leftover
    /// space. Same meaning in a Flex item's `size` and a Grid track.
    Fr(u16),
}
impl Default for Size {
    fn default() -> Self {
        Size::Fr(1)
    }
}

impl From<u16> for Size {
    fn from(v: u16) -> Self {
        Size::Length(v)
    }
}
impl From<i32> for Size {
    fn from(v: i32) -> Self {
        Size::Length(v.max(0) as u16)
    }
}
impl From<&str> for Size {
    fn from(s: &str) -> Self {
        parse_size(s)
    }
}
impl From<String> for Size {
    fn from(s: String) -> Self {
        parse_size(&s)
    }
}

pub fn parse_size(s: &str) -> Size {
    let s = s.trim();
    if s.eq_ignore_ascii_case("auto") {
        Size::Auto
    } else if let Some(pct) = s.strip_suffix('%') {
        Size::Percent(pct.trim().parse().unwrap_or(0))
    } else if let Some(fr) = s.strip_suffix("fr") {
        Size::Fr(fr.trim().parse().unwrap_or(1).max(1))
    } else {
        Size::Length(s.trim().parse().unwrap_or(0))
    }
}

pub fn parse_size_list(spec: &str) -> Vec<Size> {
    let spec = spec.trim();
    if spec.is_empty() {
        return Vec::new();
    }
    if spec.contains(',') {
        spec.split(',').map(parse_size).collect()
    } else {
        spec.split_whitespace().map(parse_size).collect()
    }
}

pub trait IntoSizeList {
    fn into_size_list(self) -> Vec<Size>;
}

impl IntoSizeList for Vec<Size> {
    fn into_size_list(self) -> Vec<Size> {
        self
    }
}

impl IntoSizeList for &[Size] {
    fn into_size_list(self) -> Vec<Size> {
        self.to_vec()
    }
}

impl IntoSizeList for &str {
    fn into_size_list(self) -> Vec<Size> {
        parse_size_list(self)
    }
}

impl IntoSizeList for String {
    fn into_size_list(self) -> Vec<Size> {
        parse_size_list(&self)
    }
}

impl IntoSizeList for usize {
    fn into_size_list(self) -> Vec<Size> {
        vec![Size::Fr(1); self]
    }
}

impl IntoSizeList for u16 {
    fn into_size_list(self) -> Vec<Size> {
        vec![Size::Fr(1); self as usize]
    }
}

impl IntoSizeList for i32 {
    fn into_size_list(self) -> Vec<Size> {
        vec![Size::Fr(1); self.max(0) as usize]
    }
}

/// Shared resolver for both Grid tracks and Flex's basis pass.
pub fn resolve_sizes(units: &[Size], available: u16, auto_sizes: &[u16]) -> Vec<u16> {
    let mut sizes = vec![0u16; units.len()];
    let mut used = 0u16;
    let mut fr_total: u32 = 0;
    for (i, unit) in units.iter().enumerate() {
        let size = match unit {
            Size::Length(n) => *n,
            Size::Percent(p) => ((u32::from(available) * u32::from(*p)) / 100) as u16,
            Size::Auto => auto_sizes.get(i).copied().unwrap_or(0),
            Size::Fr(f) => {
                fr_total += u32::from(*f);
                0
            }
        };
        let clamped = size.min(available.saturating_sub(used));
        sizes[i] = clamped;
        used = used.saturating_add(clamped);
    }
    if let Some(fr_total) = std::num::NonZeroU32::new(fr_total) {
        let remaining = available.saturating_sub(used);
        let mut fr_used = 0u16;
        for (i, unit) in units.iter().enumerate() {
            if let Size::Fr(f) = unit {
                let raw = (u32::from(remaining) * u32::from(*f)) / fr_total.get();
                let size = (raw as u16).min(remaining.saturating_sub(fr_used));
                sizes[i] = size;
                fr_used = fr_used.saturating_add(size);
            }
        }
        let mut leftover = remaining.saturating_sub(fr_used);
        for (i, unit) in units.iter().enumerate() {
            if leftover == 0 {
                break;
            }
            if matches!(unit, Size::Fr(_)) {
                sizes[i] = sizes[i].saturating_add(1);
                leftover -= 1;
            }
        }
    }
    sizes
}
