//! Shared track-sizing model used by both `Flex`'s `layout` spec and
//! `Grid`'s `columns`/`rows` tracks: fixed lengths, percentages, `fr`
//! shares of leftover space, and `auto` (content-driven) tracks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackSize {
    Length(u16),
    Percent(u16),
    Fr(u16),
    /// Size to the intrinsic content of whatever occupies this track.
    /// Resolving this requires a pre-measurement pass — see `resolve_track_sizes`.
    Auto,
}

pub fn parse_track_unit(s: &str) -> TrackSize {
    let s = s.trim();
    if s.eq_ignore_ascii_case("auto") {
        TrackSize::Auto
    } else if let Some(pct) = s.strip_suffix('%') {
        TrackSize::Percent(pct.trim().parse().unwrap_or(0))
    } else if let Some(fr) = s.strip_suffix("fr") {
        TrackSize::Fr(fr.trim().parse().unwrap_or(1).max(1))
    } else {
        TrackSize::Length(s.trim().parse().unwrap_or(0))
    }
}

pub fn parse_track_list(spec: &str) -> Vec<TrackSize> {
    spec.split(',').map(parse_track_unit).collect()
}

/// Resolve a list of tracks against `available` space.
/// `auto_sizes[i]` supplies the pre-measured content size to use for
/// `TrackSize::Auto` at index `i` (0 if that track wasn't measured, e.g.
/// it has no items).
pub fn resolve_track_sizes(units: &[TrackSize], available: u16, auto_sizes: &[u16]) -> Vec<u16> {
    let mut sizes = vec![0u16; units.len()];
    let mut used = 0u16;

    // Pass 1: fixed lengths, percentages, and auto (already-measured)
    // tracks consume space first.
    let mut fr_total: u32 = 0;
    for (i, unit) in units.iter().enumerate() {
        let size = match unit {
            TrackSize::Length(n) => *n,
            TrackSize::Percent(p) => ((u32::from(available) * u32::from(*p)) / 100) as u16,
            TrackSize::Auto => auto_sizes.get(i).copied().unwrap_or(0),
            TrackSize::Fr(f) => {
                fr_total += u32::from(*f);
                0
            }
        };
        let clamped = size.min(available.saturating_sub(used));
        sizes[i] = clamped;
        used = used.saturating_add(clamped);
    }

    // Pass 2: whatever's left is divided among `fr` tracks proportionally.
    if fr_total > 0 {
        let remaining = available.saturating_sub(used);
        let mut fr_used = 0u16;
        for (i, unit) in units.iter().enumerate() {
            if let TrackSize::Fr(f) = unit {
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
            if matches!(unit, TrackSize::Fr(_)) {
                sizes[i] = sizes[i].saturating_add(1);
                leftover -= 1;
            }
        }
    }

    sizes
}
