// crates/reactatui/src/layout/tracks.rs — replaces both TrackSize and FlexBasis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Size {
    /// Size to intrinsic content. This is the ONLY default, everywhere —
    /// nothing grows unless you say `1fr`. No more "Auto secretly means grow".
    Auto,
    Length(u16),
    Percent(u16),
    /// The one and only way to grow: a proportional share of leftover
    /// space. Same meaning in a Flex item's `size` and a Grid track.
    Fr(u16),
}
impl Default for Size {
    fn default() -> Self {
        Size::Auto
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
    spec.split(',').map(parse_size).collect()
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
    if fr_total > 0 {
        let remaining = available.saturating_sub(used);
        let mut fr_used = 0u16;
        for (i, unit) in units.iter().enumerate() {
            if let Size::Fr(f) = unit {
                let raw = (u32::from(remaining) * u32::from(*f)) / fr_total;
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
