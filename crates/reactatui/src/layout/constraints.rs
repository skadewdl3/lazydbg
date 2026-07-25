//! Track-sizing shared by `Flex`'s `layout` spec and `Grid`'s
//! `columns`/`rows` specs. Adds an `auto` unit on top of the
//! Length/Percent/Fr units `flex.rs` already had.

#[derive(Debug, Clone, Copy)]
pub enum TrackUnit {
    Length(u16),
    Percent(u16),
    Fr(u16),
    Auto,
}

pub fn parse_track_unit(s: &str) -> TrackUnit {
    let s = s.trim();
    if s.eq_ignore_ascii_case("auto") {
        TrackUnit::Auto
    } else if let Some(pct) = s.strip_suffix('%') {
        TrackUnit::Percent(pct.trim().parse().unwrap_or(0))
    } else if let Some(fr) = s.strip_suffix("fr") {
        TrackUnit::Fr(fr.trim().parse().unwrap_or(1).max(1))
    } else {
        TrackUnit::Length(s.trim().parse().unwrap_or(0))
    }
}

pub fn parse_tracks(spec: &str) -> Vec<TrackUnit> {
    spec.split(',').map(parse_track_unit).collect()
}

/// Resolve final track sizes given `available` space. For `Auto` tracks,
/// `auto_content[i]` supplies the tightest content size of the
/// smallest non-spanning child in that track (0 if none / not measured).
pub fn resolve_track_sizes(units: &[TrackUnit], auto_content: &[u16], available: u16) -> Vec<u16> {
    let mut sizes = vec![0u16; units.len()];
    let mut used = 0u16;
    let mut fr_total: u32 = 0;

    for (i, unit) in units.iter().enumerate() {
        let size = match unit {
            TrackUnit::Length(n) => *n,
            TrackUnit::Percent(p) => ((u32::from(available) * u32::from(*p)) / 100) as u16,
            TrackUnit::Auto => *auto_content.get(i).unwrap_or(&0),
            TrackUnit::Fr(f) => {
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
            if let TrackUnit::Fr(f) = unit {
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
            if matches!(unit, TrackUnit::Fr(_)) {
                sizes[i] = sizes[i].saturating_add(1);
                leftover -= 1;
            }
        }
    }

    sizes
}
