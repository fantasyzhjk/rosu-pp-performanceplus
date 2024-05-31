use crate::{
    any::difficulty::{
        object::IDifficultyObject,
        skills::{strain_decay, ISkill, Skill},
    },
    osu::{difficulty::object::OsuDifficultyObject, object::OsuObjectKind},
    util::{pplus, strains_vec::StrainsVec},
};

use super::strain::OsuStrainSkill;

const SKILL_MULTIPLIER: f64 = 2600.0 * 0.3;
const STRAIN_DECAY_BASE: f64 = 0.45;

#[derive(Clone)]
pub struct RhythmComplexity {
    difficulty_total: f64,
    circle_count: i32,
    is_previous_offbeat: bool,
    prev_doubles: Vec<i32>,
}

impl RhythmComplexity {
    pub fn new() -> Self {
        Self {
            difficulty_total: 0.0,
            circle_count: 0,
            is_previous_offbeat: false,
            prev_doubles: Vec::new(),
        }
    }

    pub fn difficulty_value(self) -> f64 {
        self.as_difficulty_value()
    }

    /// Use [`difficulty_value`] instead whenever possible because
    /// [`as_difficulty_value`] clones internally.
    pub fn as_difficulty_value(&self) -> f64 {
        let length_requirement = (self.circle_count as f64 / 50.0).tanh();
        1.0 + self.difficulty_total / self.circle_count as f64 * length_requirement
    }
}

impl ISkill for RhythmComplexity {
    type DifficultyObjects<'a> = [OsuDifficultyObject<'a>];
}

impl<'a> Skill<'a, RhythmComplexity> {
    pub fn process(&mut self, curr: &'a OsuDifficultyObject<'a>) {
        if curr.base.is_circle() {
            self.inner.difficulty_total += self.calc_rhythm_bonus(curr);
            self.inner.circle_count += 1;
        } else {
            self.inner.is_previous_offbeat = false;
        }
    }

    fn calc_rhythm_bonus(&mut self, curr: &'a OsuDifficultyObject<'a>) -> f64 {
        let mut rhythm_bonus = 0.05 * curr.flow;
        let prev = curr.previous(0, self.diff_objects);

        if let Some(prev) = prev {
            match prev.base.kind {
                OsuObjectKind::Circle => rhythm_bonus += self.calc_circle_to_circle_rhythm_bonus(curr, prev),
                OsuObjectKind::Slider(_) => rhythm_bonus += self.calc_slider_to_circle_rhythm_bonus(curr),
                OsuObjectKind::Spinner(_) => self.inner.is_previous_offbeat = false,
            }
        }

        rhythm_bonus
    }

    fn calc_circle_to_circle_rhythm_bonus(
        &mut self,
        curr: &'a OsuDifficultyObject<'a>,
        prev: &'a OsuDifficultyObject<'a>,
    ) -> f64 {
        if pplus::is_ratio_equal(0.667, curr.travel_time, prev.travel_time) && curr.flow > 0.8 {
            self.inner.is_previous_offbeat = true;
        } else if pplus::is_ratio_equal(1.0, curr.travel_time, prev.travel_time) && curr.flow > 0.8
        {
            self.inner.is_previous_offbeat = !self.inner.is_previous_offbeat;
        } else {
            self.inner.is_previous_offbeat = false;
        }

        if self.inner.is_previous_offbeat
            && pplus::is_ratio_equal_greater(1.5, curr.travel_time, prev.travel_time)
        {
            let mut rhythm_bonus = 5.0;
            for &prev_double in self
                .inner
                .prev_doubles
                .iter()
                .skip((self.inner.prev_doubles.len() - 10).max(0))
            {
                if prev_double > 0 {
                    rhythm_bonus *= 1.0 - 0.5 * (curr.idx as f64 - prev_double as f64).powf(0.9)
                } else {
                    rhythm_bonus = 5.0;
                }
            }
            self.inner.prev_doubles.push(curr.idx as i32);
            rhythm_bonus
        } else if pplus::is_ratio_equal(0.667, curr.travel_time, prev.travel_time) {
            if curr.flow > 0.8 {
                self.inner.prev_doubles.push(-1);
            }
            4.0 + 8.0 * curr.flow
        } else if pplus::is_ratio_equal(0.333, curr.travel_time, prev.travel_time) {
            0.4 + 0.8 * curr.flow
        } else if pplus::is_ratio_equal(0.5, curr.travel_time, prev.travel_time)
            || pplus::is_ratio_equal(0.25, curr.travel_time, prev.travel_time)
        {
            0.1 + 0.2 * curr.flow
        } else {
            0.0
        }
    }

    fn calc_slider_to_circle_rhythm_bonus(&mut self, curr: &'a OsuDifficultyObject<'a>) -> f64 {
        let slider_ms = curr.strain_time - curr.travel_time;

        if pplus::is_ratio_equal(0.5, curr.travel_time, slider_ms)
            || pplus::is_ratio_equal(0.25, curr.travel_time, slider_ms)
        {
            let end_flow = Self::calc_slider_end_flow(curr);

            if end_flow > 0.8 {
                self.inner.is_previous_offbeat = true;
            } else {
                self.inner.is_previous_offbeat = false;
            }

            0.3 * end_flow
        } else {
            self.inner.is_previous_offbeat = false;

            0.0
        }
    }

    fn calc_slider_end_flow(curr: &'a OsuDifficultyObject<'a>) -> f64 {
        let stream_bpm = 15000.0 / curr.travel_time;
        let is_flow_speed = pplus::transition_to_true(stream_bpm, 120.0, 30.0);
        let distance_offset =
            (((stream_bpm - 140.0) / 20.0).tanh() + 2.0) * OsuDifficultyObject::NORMALIZED_RADIUS;
        let is_flow_distance = pplus::transition_to_false(
            curr.jump_dist,
            distance_offset,
            OsuDifficultyObject::NORMALIZED_RADIUS,
        );

        is_flow_speed * is_flow_distance
    }
}
