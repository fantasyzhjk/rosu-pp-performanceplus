use crate::Difficulty;

use super::{
    convert::OsuBeatmap,
    difficulty::{skills::OsuSkills, DifficultyValues},
};

/// The result of calculating the strains on a osu! map.
///
/// Suitable to plot the difficulty of a map over time.
#[derive(Clone, Debug, PartialEq)]
pub struct OsuStrains {
    /// Strain peaks of the aim skill.
    pub aim: Vec<f64>,
    pub jump: Vec<f64>,
    pub flow: Vec<f64>,
    pub raw: Vec<f64>,
    pub speed: Vec<f64>,
    pub stamina: Vec<f64>,
}

impl OsuStrains {
    /// Time between two strains in ms.
    pub const SECTION_LEN: f64 = 400.0;
}

pub fn strains(difficulty: &Difficulty, converted: &OsuBeatmap<'_>) -> OsuStrains {
    let DifficultyValues {
        skills:
            OsuSkills {
                aim,
                flow_aim,
                jump_aim,
                raw_aim,
                speed,
                stamina,
                ..
            },
        attrs: _,
    } = DifficultyValues::calculate(difficulty, converted);

    OsuStrains {
        aim: aim.get_curr_strain_peaks().into_vec(),
        jump: jump_aim.get_curr_strain_peaks().into_vec(),
        flow: flow_aim.get_curr_strain_peaks().into_vec(),
        raw: raw_aim.get_curr_strain_peaks().into_vec(),
        speed: speed.get_curr_strain_peaks().into_vec(),
        stamina: stamina.get_curr_strain_peaks().into_vec(),
    }
}
