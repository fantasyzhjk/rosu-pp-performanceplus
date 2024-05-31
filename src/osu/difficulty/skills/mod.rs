use rhythm::RhythmComplexity;
use stamina::Stamina;

use crate::{model::beatmap::BeatmapAttributes, osu::object::OsuObject, util::mods::Mods};

use self::{aim::Aim, speed::Speed};

use super::{scaling_factor::ScalingFactor, HD_FADE_IN_DURATION_MULTIPLIER};

pub mod aim;
pub mod speed;
pub mod strain;
pub mod rhythm;
pub mod stamina;

pub struct OsuSkills {
    pub aim: Aim,
    pub flow_aim: Aim,
    pub jump_aim: Aim,
    pub raw_aim: Aim,
    pub speed: Speed,
    pub stamina: Stamina,
    pub rhythm: RhythmComplexity,
}

impl OsuSkills {
    pub fn new(
        mods: u32,
        scaling_factor: &ScalingFactor,
        map_attrs: &BeatmapAttributes,
        time_preempt: f64,
    ) -> Self {
        // let hit_window = 2.0 * map_attrs.hit_windows.od;

        // * Preempt time can go below 450ms. Normally, this is achieved via the DT mod
        // * which uniformly speeds up all animations game wide regardless of AR.
        // * This uniform speedup is hard to match 1:1, however we can at least make
        // * AR>10 (via mods) feel good by extending the upper linear function above.
        // * Note that this doesn't exactly match the AR>10 visuals as they're
        // * classically known, but it feels good.
        // * This adjustment is necessary for AR>10, otherwise TimePreempt can
        // * become smaller leading to hitcircles not fully fading in.
        let time_fade_in = if mods.hd() {
            time_preempt * HD_FADE_IN_DURATION_MULTIPLIER
        } else {
            400.0 * (time_preempt / OsuObject::PREEMPT_MIN).min(1.0)
        };

        let aim = Aim::new(scaling_factor.radius, time_preempt, time_fade_in, mods, false, false, false);
        let flow_aim = Aim::new(scaling_factor.radius, time_preempt, time_fade_in, mods, true, false, false);
        let jump_aim = Aim::new(scaling_factor.radius, time_preempt, time_fade_in, mods, false, true, false);
        let raw_aim = Aim::new(scaling_factor.radius, time_preempt, time_fade_in, mods, false, false, true);
        let speed = Speed::new();
        let stamina = Stamina::new();
        let rhythm = RhythmComplexity::new();

        Self {
            aim,
            flow_aim,
            jump_aim,
            raw_aim,
            speed,
            stamina,
            rhythm
        }
    }
}
