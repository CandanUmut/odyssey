use glam::Vec3;
use serde::{Deserialize, Serialize};

use crate::{RegionId, REGION_MARKERS, REGION_NAMES};

pub fn region_for_position(position: Vec3) -> RegionId {
    let x = position.x.max(0.0);
    match x {
        v if v < REGION_MARKERS[1] => RegionId::Vagina,
        v if v < REGION_MARKERS[2] => RegionId::Cervix,
        v if v < REGION_MARKERS[3] => RegionId::Uterus,
        v if v < REGION_MARKERS[4] => RegionId::Utj,
        v if v < REGION_MARKERS[5] => RegionId::Tube,
        _ => RegionId::Ampulla,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegionTooltip {
    pub id: RegionId,
    pub title: &'static str,
    pub note: &'static str,
}

pub fn tooltips() -> Vec<RegionTooltip> {
    vec![
        RegionTooltip {
            id: RegionId::Vagina,
            title: REGION_NAMES[0],
            note: "Acidic environment; strong swimmers push through.",
        },
        RegionTooltip {
            id: RegionId::Cervix,
            title: REGION_NAMES[1],
            note: "Dense mucus strands; alignment matters.",
        },
        RegionTooltip {
            id: RegionId::Uterus,
            title: REGION_NAMES[2],
            note: "Uterine contractions can help or hinder.",
        },
        RegionTooltip {
            id: RegionId::Utj,
            title: REGION_NAMES[3],
            note: "Narrow gate; many sperm fail to pass.",
        },
        RegionTooltip {
            id: RegionId::Tube,
            title: REGION_NAMES[4],
            note: "Fluid flow guides swimmers toward the ampulla.",
        },
        RegionTooltip {
            id: RegionId::Ampulla,
            title: REGION_NAMES[5],
            note: "Fertilization awaits the fastest explorers.",
        },
    ]
}

pub fn tube_radius(region: RegionId) -> f32 {
    match region {
        RegionId::Vagina => 160.0,
        RegionId::Cervix => 120.0,
        RegionId::Uterus => 150.0,
        RegionId::Utj => 90.0,
        RegionId::Tube => 110.0,
        RegionId::Ampulla => 140.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn region_progression() {
        let checkpoints = [
            (Vec3::new(10.0, 0.0, 0.0), RegionId::Vagina),
            (
                Vec3::new(REGION_MARKERS[1] + 1.0, 0.0, 0.0),
                RegionId::Cervix,
            ),
            (
                Vec3::new(REGION_MARKERS[2] + 1.0, 0.0, 0.0),
                RegionId::Uterus,
            ),
            (Vec3::new(REGION_MARKERS[3] + 1.0, 0.0, 0.0), RegionId::Utj),
            (Vec3::new(REGION_MARKERS[4] + 1.0, 0.0, 0.0), RegionId::Tube),
            (
                Vec3::new(REGION_MARKERS[5] + 10.0, 0.0, 0.0),
                RegionId::Ampulla,
            ),
        ];

        for (pos, expected) in checkpoints {
            assert_eq!(region_for_position(pos), expected);
        }
    }
}
