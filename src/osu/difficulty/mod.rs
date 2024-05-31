use std::{borrow::Borrow, cmp, ops::Deref, pin::Pin};

use crate::{
    any::difficulty::{skills::Skill, Difficulty},
    model::beatmap::BeatmapAttributes,
    osu::{
        convert::convert_objects,
        difficulty::{object::OsuDifficultyObject, scaling_factor::ScalingFactor},
        object::OsuObject,
        performance::PERFORMANCE_BASE_MULTIPLIER,
    },
    util::mods::Mods,
};

use self::skills::OsuSkills;

use super::{attributes::OsuDifficultyAttributes, convert::OsuBeatmap};

pub mod gradual;
mod object;
pub mod scaling_factor;
pub mod skills;

const DIFFICULTY_MULTIPLIER: f64 = 0.0675;

const HD_FADE_IN_DURATION_MULTIPLIER: f64 = 0.4;
const HD_FADE_OUT_DURATION_MULTIPLIER: f64 = 0.3;

pub fn difficulty(difficulty: &Difficulty, converted: &OsuBeatmap<'_>) -> OsuDifficultyAttributes {
    let DifficultyValues {
        skills:
            OsuSkills { aim, flow_aim, jump_aim, raw_aim, speed, stamina, rhythm },
        mut attrs,
    } = DifficultyValues::calculate(difficulty, converted);

    let aim_difficulty_value = aim.difficulty_value();
    let flow_aim_difficulty_value = flow_aim.difficulty_value();
    let jump_aim_difficulty_value = jump_aim.difficulty_value();
    let raw_aim_difficulty_value = raw_aim.difficulty_value();
    let stamina_difficulty_value = stamina.difficulty_value();
    let rhythm_difficulty_value = rhythm.difficulty_value();
    let speed_difficulty_value = speed.difficulty_value();

    // let mods = difficulty.get_mods();

    DifficultyValues::eval(
        &mut attrs,
        aim_difficulty_value,
        flow_aim_difficulty_value,
        jump_aim_difficulty_value,
        raw_aim_difficulty_value,
        stamina_difficulty_value,
        rhythm_difficulty_value,
        speed_difficulty_value,
    );

    attrs
}

pub struct OsuDifficultySetup {
    scaling_factor: ScalingFactor,
    map_attrs: BeatmapAttributes,
    attrs: OsuDifficultyAttributes,
    time_preempt: f64,
}

impl OsuDifficultySetup {
    pub fn new(difficulty: &Difficulty, converted: &OsuBeatmap) -> Self {
        let clock_rate = difficulty.get_clock_rate();
        let map_attrs = converted.attributes().difficulty(difficulty).build();
        let scaling_factor = ScalingFactor::new(map_attrs.cs);

        let attrs = OsuDifficultyAttributes {
            ar: map_attrs.ar,
            hp: map_attrs.hp,
            od: map_attrs.od,
            ..Default::default()
        };

        let time_preempt = f64::from((map_attrs.hit_windows.ar * clock_rate) as f32);

        Self {
            scaling_factor,
            map_attrs,
            attrs,
            time_preempt,
        }
    }
}

pub struct DifficultyValues {
    pub skills: OsuSkills,
    pub attrs: OsuDifficultyAttributes,
}

impl DifficultyValues {
    pub fn calculate(difficulty: &Difficulty, converted: &OsuBeatmap<'_>) -> Self {
        let mods = difficulty.get_mods();
        let take = difficulty.get_passed_objects();

        let OsuDifficultySetup {
            scaling_factor,
            map_attrs,
            mut attrs,
            time_preempt,
        } = OsuDifficultySetup::new(difficulty, converted);

        let mut osu_objects = convert_objects(
            converted,
            &scaling_factor,
            mods.hr(),
            time_preempt,
            take,
            &mut attrs,
        );

        let osu_object_iter = osu_objects.iter_mut().map(Pin::new);

        let diff_objects =
            Self::create_difficulty_objects(difficulty, &scaling_factor, osu_object_iter, time_preempt);

        let mut skills = OsuSkills::new(mods, &scaling_factor, &map_attrs, time_preempt);

        {
            let mut aim = Skill::new(&mut skills.aim, &diff_objects);
            let mut flow_aim = Skill::new(&mut skills.flow_aim, &diff_objects);
            let mut jump_aim = Skill::new(&mut skills.jump_aim, &diff_objects);
            let mut raw_aim = Skill::new(&mut skills.raw_aim, &diff_objects);
            let mut stamina = Skill::new(&mut skills.stamina, &diff_objects);
            let mut rhythm = Skill::new(&mut skills.rhythm, &diff_objects);
            let mut speed = Skill::new(&mut skills.speed, &diff_objects);

            // The first hit object has no difficulty object
            let take_diff_objects = cmp::min(converted.hit_objects.len(), take).saturating_sub(1);

            for hit_object in diff_objects.iter().take(take_diff_objects) {
                aim.process(hit_object);
                raw_aim.process(hit_object);
                jump_aim.process(hit_object);
                flow_aim.process(hit_object);
                stamina.process(hit_object);
                rhythm.process(hit_object);
                speed.process(hit_object);
            }
        }

        Self { skills, attrs }
    }

    /// Process the difficulty values and store the results in `attrs`.
    pub fn eval(
        attrs: &mut OsuDifficultyAttributes,
        aim_difficulty_value: f64,
        flow_aim_difficulty_value: f64,
        jump_aim_difficulty_value: f64,
        raw_aim_difficulty_value: f64,
        stamina_difficulty_value: f64,
        rhythm_difficulty_value: f64,
        speed_difficulty_value: f64,
    ) {
        let aim_rating = aim_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let jump_aim_rating = jump_aim_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let flow_aim_rating = flow_aim_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let precision_rating = (aim_difficulty_value - raw_aim_difficulty_value).max(0.0).sqrt() * DIFFICULTY_MULTIPLIER;
        let speed_rating = speed_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let stamina_rating = stamina_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let accuracy_rating = rhythm_difficulty_value.sqrt();

        

        let star_rating = (aim_rating.powf(3.0) + speed_rating.max(stamina_rating).powf(3.0)).powf(1.0 / 3.0) * 1.6;

        attrs.stars = star_rating;
        attrs.aim = aim_rating;
        attrs.jump = jump_aim_rating;
        attrs.flow = flow_aim_rating;
        attrs.precision = precision_rating;
        attrs.stamina = stamina_rating;
        attrs.accuracy = accuracy_rating;
        attrs.speed = speed_rating;
    }

    pub fn create_difficulty_objects<'a>(
        difficulty: &Difficulty,
        scaling_factor: &ScalingFactor,
        osu_objects: impl ExactSizeIterator<Item = Pin<&'a mut OsuObject>>,
        time_preempt: f64
    ) -> Vec<OsuDifficultyObject<'a>> {
        let take = difficulty.get_passed_objects();
        let clock_rate = difficulty.get_clock_rate();

        let mut osu_objects_iter = osu_objects
            .map(|h| OsuDifficultyObject::compute_slider_cursor_pos(h, scaling_factor.radius))
            .map(Pin::into_ref);

        let Some(mut last) = osu_objects_iter.next().filter(|_| take > 0) else {
            return Vec::new();
        };

        let mut last_last = None;
        let mut last_diff_object: Option<OsuDifficultyObject> = None;
        let mut last_last_diff_object: Option<OsuDifficultyObject> = None;
  
        osu_objects_iter
            .enumerate()
            .map(|(idx, h)| {
                let diff_object = OsuDifficultyObject::new(
                    h.get_ref(),
                    last.get_ref(),
                    last_last.as_deref(),
                    last_diff_object,
                    last_last_diff_object,
                    clock_rate,
                    time_preempt,
                    idx,
                    scaling_factor,
                );

                last_last_diff_object = last_diff_object;
                last_diff_object = Some(diff_object);

                last_last = Some(last);
                last = h;

                diff_object
            })
            .collect()
    }
}
