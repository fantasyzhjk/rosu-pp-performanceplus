use std::{
    collections::VecDeque,
    f64::consts::PI,
};

use rosu_map::util::Pos;

use crate::{
    any::difficulty::{
        object::IDifficultyObject,
        skills::{strain_decay, ISkill, Skill},
    },
    osu::{difficulty::object::{OsuDifficultyNoBase, OsuDifficultyObject}, PLAYFIELD_BASE_SIZE},
    util::{mods::Mods, pplus, strains_vec::StrainsVec},
};

use super::strain::OsuStrainSkill;

const SKILL_MULTIPLIER: f64 = 1059.0;
const STRAIN_DECAY_BASE: f64 = 0.15;

#[derive(Clone)]
pub struct Aim {
    curr_strain: f64,
    inner: OsuStrainSkill,
    evaluator: AimEvaluator,
    flow_aim: bool,
    jump_aim: bool,
    raw_aim: bool,
}

impl Aim {
    pub fn new(
        radius: f64,
        time_preempt: f64,
        time_fade_in: f64,
        mods: u32,
        flow_aim: bool,
        jump_aim: bool,
        raw_aim: bool,
    ) -> Self {
        Self {
            curr_strain: 0.0,
            inner: OsuStrainSkill::default(),
            evaluator: AimEvaluator {
                radius,
                time_preempt,
                time_fade_in,
                mods,
                preempt_hit_objects: VecDeque::new(),
            },
            flow_aim,
            jump_aim,
            raw_aim,
        }
    }

    pub fn get_curr_strain_peaks(self) -> StrainsVec {
        self.inner.get_curr_strain_peaks()
    }

    pub fn difficulty_value(self) -> f64 {
        Self::static_difficulty_value(self.inner)
    }

    /// Use [`difficulty_value`] instead whenever possible because
    /// [`as_difficulty_value`] clones internally.
    pub fn as_difficulty_value(&self) -> f64 {
        Self::static_difficulty_value(self.inner.clone())
    }

    fn static_difficulty_value(skill: OsuStrainSkill) -> f64 {
        skill.difficulty_value()
    }
}

impl ISkill for Aim{
    type DifficultyObjects<'a> = [OsuDifficultyObject<'a>];
}

impl<'a> Skill<'a, Aim> {
    fn calculate_initial_strain(&mut self, time: f64, curr: &'a OsuDifficultyObject<'a>) -> f64 {
        let prev_start_time = curr
            .previous(0, self.diff_objects)
            .map_or(0.0, |prev| prev.start_time);

        self.inner.curr_strain * strain_decay(time - prev_start_time, STRAIN_DECAY_BASE)
    }

    fn curr_section_peak(&self) -> f64 {
        self.inner.inner.inner.curr_section_peak
    }

    fn curr_section_peak_mut(&mut self) -> &mut f64 {
        &mut self.inner.inner.inner.curr_section_peak
    }

    fn curr_section_end(&self) -> f64 {
        self.inner.inner.inner.curr_section_end
    }

    fn curr_section_end_mut(&mut self) -> &mut f64 {
        &mut self.inner.inner.inner.curr_section_end
    }

    pub fn process(&mut self, curr: &'a OsuDifficultyObject<'a>) {
        if curr.idx == 0 {
            *self.curr_section_end_mut() = (curr.start_time / OsuStrainSkill::SECTION_LEN).ceil()
                * OsuStrainSkill::SECTION_LEN;
        }

        while curr.start_time > self.curr_section_end() {
            self.inner.inner.save_curr_peak();
            let initial_strain = self.calculate_initial_strain(self.curr_section_end(), curr);
            self.inner.inner.start_new_section_from(initial_strain);
            *self.curr_section_end_mut() += OsuStrainSkill::SECTION_LEN;
        }

        let strain_value_at = self.strain_value_at(curr);
        *self.curr_section_peak_mut() = strain_value_at.max(self.curr_section_peak());
    }

    fn strain_value_at(&mut self, curr: &'a OsuDifficultyObject<'a>) -> f64 {
        self.inner.curr_strain *= strain_decay(curr.delta_time, STRAIN_DECAY_BASE);
        self.inner.curr_strain += self.inner.evaluator.evaluate_diff_of(
            curr,
            self.diff_objects,
            self.inner.flow_aim,
            self.inner.jump_aim,
            self.inner.raw_aim,
        ) * SKILL_MULTIPLIER;

        self.inner.curr_strain
    }
}

#[derive(Clone)]
struct AimEvaluator {
    time_preempt: f64,
    time_fade_in: f64,
    radius: f64,
    mods: u32,
    preempt_hit_objects: VecDeque<OsuDifficultyNoBase>,
}

impl AimEvaluator {
    fn evaluate_diff_of<'a>(
        &mut self,
        curr: &'a OsuDifficultyObject<'a>,
        diff_objects: &'a [OsuDifficultyObject<'a>],
        flow_aim: bool,
        jump_aim: bool,
        raw_aim: bool,
    ) -> f64 {
        let osu_curr_obj = curr;

        let prev2s: Vec<OsuDifficultyObject> = curr
            .previous(0, diff_objects)
            .into_iter()
            .chain(curr.previous(1, diff_objects)).copied()
            .collect();

        let aim = if flow_aim {
            Self::calc_flow_aim_value(osu_curr_obj, prev2s.first())
                * Self::calc_small_circle_bonus(self.radius)
        } else if jump_aim {
            Self::calc_jump_aim_value(osu_curr_obj, &prev2s, false)
                * Self::calc_small_circle_bonus(self.radius)
        } else if raw_aim {
            Self::calc_flow_aim_value(osu_curr_obj, prev2s.first())
                + Self::calc_jump_aim_value(osu_curr_obj, &prev2s, true)
        } else {
            (Self::calc_flow_aim_value(osu_curr_obj, prev2s.first())
                + Self::calc_jump_aim_value(osu_curr_obj, &prev2s, false))
                * Self::calc_small_circle_bonus(self.radius)
        };
        
        let reading_multiplier = Self::calc_reading_multiplier(
            &mut self.preempt_hit_objects,
            osu_curr_obj,
            self.mods.hd(),
            self.mods.fl(),
            self.radius,
        );

        aim * reading_multiplier
    }

    fn calc_jump_aim_value(
        curr: &OsuDifficultyObject,
        prev2s: &[OsuDifficultyObject],
        raw: bool,
    ) -> f64 {
        if (curr.flow - 1.0).abs() < f64::EPSILON {
            return 0.0;
        };

        let distance = if raw {
            curr.raw_jump_dist
        } else {
            curr.jump_dist
        } / OsuDifficultyObject::NORMALIZED_RADIUS;

        let jump_aim_base = distance / curr.strain_time;

        let pattern_weight = Self::calc_jump_pattern_weight(curr, prev2s);

        let (location_weight, angle_weight) = if let Some(prev) = prev2s.iter().next() {
            (
                Self::calc_location_weight(curr.base.pos, prev.base.pos),
                Self::calc_jump_angle_weight(
                    curr.angle,
                    curr.strain_time,
                    prev.strain_time,
                    prev.jump_dist,
                ),
            )
        } else {
            (
                1.0,
                Self::calc_jump_angle_weight(curr.angle, curr.strain_time, 0.0, 0.0),
            )
        };

        let jump_aim = jump_aim_base * angle_weight * pattern_weight * location_weight;
        jump_aim * (1.0 - curr.flow)
    }

    fn calc_flow_aim_value(curr: &OsuDifficultyObject, prev: Option<&OsuDifficultyObject>) -> f64 {
        if curr.flow == 0.0 {
            return 0.0;
        };

        let distance = curr.jump_dist / OsuDifficultyObject::NORMALIZED_RADIUS;

        // The 1.9 exponent roughly equals the inherent BPM based scaling the strain mechanism adds in the relevant BPM range.
        // This way the aim value of streams stays more or less consistent for a given velocity.
        // (300 BPM 20 spacing compared to 150 BPM 40 spacing for example.)
        let flow_aim_base = (1.0 + (distance - 2.0).tanh()) * 2.5 / curr.strain_time
            + (distance / 5.0) / curr.strain_time;

        
            
        let angle_weight = Self::calc_flow_angle_weight(curr.angle);
        let pattern_weight = Self::calc_flow_pattern_weight(curr, prev, distance);
        let location_weight = if let Some(prev) = prev {
            Self::calc_location_weight(curr.base.pos, prev.base.pos)
        } else {
            1.0
        };


        let flow_aim =
            flow_aim_base * angle_weight * pattern_weight * (1.0 + (location_weight - 1.0) / 2.0);
        flow_aim * curr.flow
    }

    fn calc_reading_multiplier<'a>(
        preempt_hit_objects: &mut VecDeque<OsuDifficultyNoBase>,
        curr: &'a OsuDifficultyObject<'a>,
        has_hidden: bool,
        has_fl: bool,
        radius: f64,
    ) -> f64 {
        while !preempt_hit_objects.is_empty()
            && preempt_hit_objects.front().unwrap().start_time < curr.start_time - curr.preempt
        {
            preempt_hit_objects.pop_front();
        }

        let mut reading_strain = 0.0;
        for prev in preempt_hit_objects.iter() {
            reading_strain += Self::calc_reading_density(prev.base_flow, prev.jump_dist);
        }

        // ~10-15% relative aim bonus at higher density values.
        let density_bonus = reading_strain.powf(1.5) / 100.0;

        let reading_multiplier = if has_hidden {
            1.05 + density_bonus * 1.5 // 5% flat aim bonus and density bonus increased by 50%.
        } else {
            1.0 + density_bonus
        };

        let flashlight_multiplier =
            Self::calc_flashlight_multiplier(has_fl, curr.raw_jump_dist, radius);
        let high_approach_rate_multiplier = Self::calc_high_ar_multiplier(curr.preempt);

        preempt_hit_objects.push_back(OsuDifficultyNoBase::from(*curr));

        reading_multiplier * flashlight_multiplier * high_approach_rate_multiplier
    }

    fn calc_jump_pattern_weight(curr: &OsuDifficultyObject, prev2s: &[OsuDifficultyObject]) -> f64 {
        let mut jump_pattern_weight = 1.0;
        for (i, previous_object) in prev2s.iter().enumerate() {
            let mut velocity_weight = 1.05;
            if previous_object.jump_dist > 0.0 {
                let velocity_ratio = (curr.jump_dist / curr.strain_time)
                    / (previous_object.jump_dist / previous_object.strain_time)
                    - 1.0;
                if velocity_ratio <= 0.0 {
                    velocity_weight = 1.0 + velocity_ratio * velocity_ratio / 2.0;
                } else if velocity_ratio < 1.0 {
                    velocity_weight =
                        1.0 + (-((velocity_ratio * PI).cos()) + 1.0) / 40.0;
                }
            }

            let mut angle_weight = 1.0;
            if pplus::is_ratio_equal(1.0, curr.strain_time, previous_object.strain_time)
                && !pplus::is_null_or_nan(curr.angle)
                && !pplus::is_null_or_nan(previous_object.angle)
            {
                let angle_change =
                    (curr.angle.unwrap().abs() - previous_object.angle.unwrap().abs()).abs();
                if angle_change >= PI / 1.5 {
                    angle_weight = 1.05;
                } else {
                    angle_weight = 1.0
                        + (-((angle_change * 1.5).cos() * PI / 2.0).sin() + 1.0)
                            / 40.0;
                }
            }

            jump_pattern_weight *= (velocity_weight * angle_weight).powf(2.0 - i as f64);
        }

        let mut distance_requirement = 0.0;
        if let Some(prev) = prev2s.iter().next() {
            distance_requirement =
                Self::calc_distance_requirement(curr.strain_time, prev.strain_time, prev.jump_dist);
        }

        1.0 + (jump_pattern_weight - 1.0) * distance_requirement
    }

    fn calc_flow_pattern_weight(
        curr: &OsuDifficultyObject,
        prev: Option<&OsuDifficultyObject>,
        distance: f64,
    ) -> f64 {
        if let Some(prev) = prev {
            let distance_rate = if prev.jump_dist > 0.0 {
                curr.jump_dist / prev.jump_dist - 1.0
            } else {
                1.0
            };

            let distance_bonus = if distance_rate <= 0.0 {
                distance_rate * distance_rate
            } else if distance_rate < 1.0 {
                (-((PI * distance_rate).cos()) + 1.0) / 2.0
            } else {
                1.0
            };



            let angle_bonus = if !pplus::is_null_or_nan(curr.angle)
                && !pplus::is_null_or_nan(prev.angle)
            {
                let (cangle, pangle) = (curr.angle.unwrap(), prev.angle.unwrap());
                let mut angle_bonus = 0.0;
                if cangle > 0.0 && pangle < 0.0 || cangle < 0.0 && pangle > 0.0 {
                    let angle_change = if cangle.abs() > (PI - pangle.abs()) / 2.0
                    {
                        PI - cangle.abs()
                    } else {
                        pangle.abs() - cangle.abs()
                    };
                    angle_bonus =
                        (-((angle_change / 2.0).sin() * PI).cos() + 1.0) / 2.0;
                } else if cangle.abs() < pangle.abs() {
                    let angle_change = cangle - pangle;
                    angle_bonus =
                        (-((angle_change / 2.0).sin() * PI).cos() + 1.0) / 2.0;
                };

                if angle_bonus > 0.0 {
                    let angle_change = cangle.abs() - pangle.abs();
                    angle_bonus =
                        ((-((angle_change / 2.0).sin() * PI).cos() + 1.0) / 2.0)
                            .min(angle_bonus);
                }

                angle_bonus
            } else {
                0.0
            };

            
            
            let stream_jump_rate = pplus::transition_to_true(distance_rate, 0.0, 1.0);
            let distance_weight = (1.0 + distance_bonus)
            * Self::calc_stream_jump_weight(curr.jump_dist, stream_jump_rate, distance);
            let angle_weight = 1.0 + angle_bonus * (1.0 - stream_jump_rate);

            1.0 + (distance_weight * angle_weight - 1.0) * prev.flow
        } else {
            1.0
        }
    }

    fn calc_jump_angle_weight(
        angle: Option<f64>,
        delta_time: f64,
        previous_delta_time: f64,
        previous_distance: f64,
    ) -> f64 {
        if let Some(angle) = angle {
            if angle.is_nan() {
                1.0
            } else {
                let distance_requirement = Self::calc_distance_requirement(
                    delta_time,
                    previous_delta_time,
                    previous_distance,
                );
                1.0 + (-((angle.cos() * PI / 2.0).sin()) + 1.0) / 10.0
                    * distance_requirement
            }
        } else {
            1.0
        }
    }

    fn calc_flow_angle_weight(angle: Option<f64>) -> f64 {
        if let Some(angle) = angle {
            if angle.is_nan() {
                1.0
            } else {
                1.0 + (angle.cos() + 1.0) / 10.0
            }
        } else {
            1.0
        }
    }

    fn calc_stream_jump_weight(jump_dist: f64, stream_jump_rate: f64, distance: f64) -> f64 {
        if jump_dist > 0.0 {
            let flow_aim_revert_factor =
                1.0 / (((distance - 2.0).tanh() + 1.0) * 2.5 + distance / 5.0);
            (1.0 - stream_jump_rate) * 1.0 + stream_jump_rate * flow_aim_revert_factor * distance
        } else {
            1.0
        }
    }

    fn calc_location_weight(pos: Pos, prev_pos: Pos) -> f64 {
        let mut x = f64::from((pos.x + prev_pos.x) * 0.5);
        let mut y = f64::from((pos.y + prev_pos.y) * 0.5);

        x -= f64::from(PLAYFIELD_BASE_SIZE.x) / 2.0;
        y -= f64::from(PLAYFIELD_BASE_SIZE.y) / 2.0;

        let angel = PI / 3.0;
        let a = (x * angel.cos() + y * angel.sin()) / 750.0;
        let b = (x * angel.sin() - y * angel.cos()) / 1000.0;

        let location_bonus = a * a + b * b;
        1.0 + location_bonus
    }

    fn calc_distance_requirement(
        delta_time: f64,
        previous_delta_time: f64,
        previous_distance: f64,
    ) -> f64 {
        if pplus::is_ratio_equal_greater(1.0, delta_time, previous_delta_time) {
            let overlap_distance =
                (previous_delta_time / delta_time) * OsuDifficultyObject::NORMALIZED_RADIUS * 2.0;
            pplus::transition_to_true(previous_distance, 0.0, overlap_distance)
        } else {
            0.0
        }
    }

    fn calc_reading_density(prev_base_flow: f64, prev_jump_dist: f64) -> f64 {
        (1.0 - prev_base_flow * 0.75)
            * (1.0
                + prev_base_flow * 0.5 * prev_jump_dist
                    / OsuDifficultyObject::NORMALIZED_RADIUS)
    }

    fn calc_flashlight_multiplier(
        flashlight_enabled: bool,
        raw_jump_distance: f64,
        radius: f64,
    ) -> f64 {
        if flashlight_enabled {
            1.0 + pplus::transition_to_true(
                raw_jump_distance,
                (PLAYFIELD_BASE_SIZE.y / 4.0).into(),
                radius,
            ) * 0.3
        } else {
            1.0
        }
    }

    fn calc_small_circle_bonus(radius: f64) -> f64 {
        1.0 + 120.0 / radius.powf(2.0)
    }

    fn calc_high_ar_multiplier(preempt: f64) -> f64 {
        1.0 + (-((preempt - 325.0) / 30.0).tanh() + 1.0) / 15.0
    }
}
