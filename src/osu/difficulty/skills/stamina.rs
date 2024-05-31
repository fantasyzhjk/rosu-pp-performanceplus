use crate::{
    any::difficulty::{
        object::IDifficultyObject,
        skills::{strain_decay, ISkill, Skill},
    },
    osu::difficulty::object::OsuDifficultyObject,
    util::strains_vec::StrainsVec,
};

use super::strain::OsuStrainSkill;

const SKILL_MULTIPLIER: f64 = 2600.0 * 0.3;
const STRAIN_DECAY_BASE: f64 = 0.45;

#[derive(Clone)]
pub struct Stamina {
    curr_strain: f64,
    inner: OsuStrainSkill,
}

impl Stamina {
    pub fn new() -> Self {
        Self {
            curr_strain: 0.0,
            inner: OsuStrainSkill::default(),
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

impl ISkill for Stamina {
    type DifficultyObjects<'a> = [OsuDifficultyObject<'a>];
}

impl<'a> Skill<'a, Stamina> {
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
        self.inner.curr_strain *= strain_decay(curr.strain_time, STRAIN_DECAY_BASE);
        self.inner.curr_strain += StaminaEvaluator::evaluate_diff_of(curr) * SKILL_MULTIPLIER;

        self.inner.curr_strain
    }
}



struct StaminaEvaluator;

impl StaminaEvaluator {
    fn evaluate_diff_of<'a>(
        curr: &'a OsuDifficultyObject<'a>,
    ) -> f64 {
        let ms = curr.last_two_strain_time / 2.0;
        
        let tap_value = 2.0 / (ms - 20.0);
        let stream_value = 1.0 / (ms - 20.0);

        (1.0 - curr.flow) * tap_value + curr.flow * stream_value
    }
}