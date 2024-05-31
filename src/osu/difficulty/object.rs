use std::pin::Pin;

use rosu_map::util::Pos;

use crate::{
    any::difficulty::object::IDifficultyObject,
    osu::object::{OsuObject, OsuObjectKind},
    util::pplus,
};

use super::{
    scaling_factor::ScalingFactor,
    HD_FADE_OUT_DURATION_MULTIPLIER,
};

#[derive(Clone, Copy)]
pub struct OsuDifficultyNoBase {
    pub idx: usize,
    pub start_time: f64,
    pub delta_time: f64,

    pub strain_time: f64,
    pub last_two_strain_time: f64,
    pub raw_jump_dist: f64,
    pub jump_dist: f64,
    pub base_flow: f64,
    pub flow: f64,
    pub travel_dist: f64,
    pub travel_time: f64,
    pub angle: Option<f64>,
    pub angle_leniency: f64,
    pub preempt: f64,
    stream_bpm: f64,
}

impl From<OsuDifficultyObject<'_>> for OsuDifficultyNoBase {
    fn from(value: OsuDifficultyObject) -> Self {
        OsuDifficultyNoBase {
            idx: value.idx,
            start_time: value.start_time,
            delta_time: value.delta_time,
            strain_time: value.strain_time,
            last_two_strain_time: value.last_two_strain_time,
            raw_jump_dist: value.raw_jump_dist,
            jump_dist: value.jump_dist,
            base_flow: value.base_flow,
            flow: value.flow,
            travel_dist: value.travel_dist,
            travel_time: value.travel_time,
            angle: value.angle,
            angle_leniency: value.angle_leniency,
            preempt: value.preempt,
            stream_bpm: value.stream_bpm,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct OsuDifficultyObject<'a> {
    pub idx: usize,
    pub base: &'a OsuObject,
    pub start_time: f64,
    pub delta_time: f64,

    pub strain_time: f64,
    pub last_two_strain_time: f64,
    pub raw_jump_dist: f64,
    pub jump_dist: f64,
    pub base_flow: f64,
    pub flow: f64,
    pub travel_dist: f64,
    pub travel_time: f64,
    pub angle: Option<f64>,
    pub angle_leniency: f64,
    pub preempt: f64,
    stream_bpm: f64,
}

impl<'a> OsuDifficultyObject<'a> {
    pub const NORMALIZED_RADIUS: f64 = 52.0;

    const MIN_DELTA_TIME: f64 = 50.0;
    const MIN_LAST_TWO_TIME: f64 = 100.0;
    const MAX_SLIDER_RADIUS: f64 = Self::NORMALIZED_RADIUS * 2.4;
    const ASSUMED_SLIDER_RADIUS: f64 = Self::NORMALIZED_RADIUS * 1.8;

    pub fn new(
        hit_object: &'a OsuObject,
        last_object: &'a OsuObject,
        last_last_object: Option<&OsuObject>,
        last_diff_object: Option<OsuDifficultyObject<'a>>,
        last_last_diff_object: Option<OsuDifficultyObject<'a>>,
        clock_rate: f64,
        time_preempt: f64,
        idx: usize,
        scaling_factor: &ScalingFactor,
    ) -> Self {
        let delta_time = (hit_object.start_time - last_object.start_time) / clock_rate;
        let start_time = hit_object.start_time / clock_rate;

        let strain_time = delta_time.max(Self::MIN_DELTA_TIME);

        let last_two_strain_time = if let Some(last_last_object) = last_last_object {
            ((hit_object.start_time - last_last_object.start_time) / clock_rate)
                .max(Self::MIN_LAST_TWO_TIME)
        } else {
            Self::MIN_LAST_TWO_TIME
        };

        let stream_bpm = 15000.0 / strain_time;
        let preempt = time_preempt / clock_rate;

        let mut this = Self {
            idx,
            base: hit_object,
            start_time,
            delta_time,
            strain_time,
            last_two_strain_time,
            raw_jump_dist: 0.0,
            jump_dist: 0.0,
            base_flow: 0.0,
            flow: 0.0,
            travel_dist: 0.0,
            travel_time: 0.0,
            angle: None,
            angle_leniency: 0.0,
            preempt,
            stream_bpm,
        };

        
        this.set_distances(last_object, last_last_object, clock_rate, scaling_factor);
        this.set_flow_values(last_diff_object, last_last_diff_object);
        
        this
    }

    pub fn set_flow_values(
        &mut self,
        last_diff_object: Option<OsuDifficultyObject>,
        last_last_diff_object: Option<OsuDifficultyObject>,
    ) {
        let mut angle_scaling_factor = None;
        let mut irregular_flow = 0.0;
        
        if let Some(last_diff_object) = last_diff_object {
            if pplus::is_ratio_equal_less(0.667, self.strain_time, last_diff_object.strain_time) {
                angle_scaling_factor = Some(1.0);
            }
            
            if pplus::is_roughly_equal(self.strain_time, last_diff_object.strain_time) {
                angle_scaling_factor = Some(if let Some(angle) = self.angle {
                    if angle.is_nan() {
                        0.5
                    } else {
                        let angle_scaling_factor =
                        (-((angle.cos() * std::f64::consts::PI / 2.0).sin()) + 3.0) / 4.0;
                        angle_scaling_factor
                        + (1.0 - angle_scaling_factor) * last_diff_object.angle_leniency
                    }
                } else {
                    0.5
                });

                let distance_offset = (((self.stream_bpm - 140.0) / 20.0).tanh() * 1.75 + 2.75) * Self::NORMALIZED_RADIUS;
                irregular_flow = pplus::transition_to_false(self.jump_dist, distance_offset, distance_offset);
                irregular_flow *= last_diff_object.base_flow;
            }
        } else {
            angle_scaling_factor = Some(1.0);
        }
        
        if let Some(last_last_diff_object) = last_last_diff_object {
            if pplus::is_roughly_equal(self.strain_time, last_last_diff_object.strain_time) {
                let distance_offset = (((self.stream_bpm - 140.0) / 20.0).tanh() * 1.75 + 2.75) * Self::NORMALIZED_RADIUS;
                irregular_flow = pplus::transition_to_false(self.jump_dist, distance_offset, distance_offset);
                irregular_flow *= last_last_diff_object.base_flow;
            }
        }

        if let Some(angle_scaling_factor) = angle_scaling_factor {
            let speed_flow = pplus::transition_to_true(self.stream_bpm, 90.0, 30.0);
            let distance_offset = (((self.stream_bpm - 140.0) / 20.0).tanh() + 2.0) * Self::NORMALIZED_RADIUS;
            self.base_flow = speed_flow * pplus::transition_to_false(self.jump_dist, distance_offset * angle_scaling_factor, distance_offset);
        } else {
            self.base_flow = 0.0;
        }

        if last_diff_object.is_some() {
            self.angle_leniency = (1.0 - self.base_flow) * irregular_flow;
            self.flow = self.base_flow.max(irregular_flow);
        } else {
            self.flow = self.base_flow;
        }

    }

    pub fn set_distances(
        &mut self,
        last_object: &OsuObject,
        last_last_object: Option<&OsuObject>,
        clock_rate: f64,
        scaling_factor: &ScalingFactor,
    ) {
        let scaling_factor = scaling_factor.factor_with_small_circle_bonus;

        if let OsuObjectKind::Circle = last_object.kind {
            self.travel_time = self.strain_time;
        }
        
        if let OsuObjectKind::Slider(ref slider) = last_object.kind {
            self.travel_dist = f64::from(slider.lazy_travel_dist * scaling_factor);
            self.travel_time =
                ((self.start_time - last_object.end_time()) / clock_rate).max(Self::MIN_DELTA_TIME);
        }

        if let OsuObjectKind::Spinner(_) = last_object.kind {
            self.travel_time =
                ((self.start_time - last_object.end_time()) / clock_rate).max(Self::MIN_DELTA_TIME);
        }

        let last_cursor_pos = Self::get_end_cursor_pos(last_object);

        if !self.base.is_spinner() {
            self.raw_jump_dist = f64::from((self.base.stacked_pos() - last_cursor_pos).length());
            self.jump_dist = self.raw_jump_dist * f64::from(scaling_factor);
        }

        if let Some(last_last_object) = last_last_object {
            let last_last_cursor_pos = Self::get_end_cursor_pos(last_last_object);

            let v1 = last_last_cursor_pos - last_object.stacked_pos();
            let v2 = self.base.stacked_pos() - last_cursor_pos;

            let dot = v1.dot(v2);
            let det = v1.x * v2.y - v1.y * v2.x;

            self.angle = Some((f64::from(det).atan2(f64::from(dot))).abs());
        }
    }

    /// The [`Pin<&mut OsuObject>`](std::pin::Pin) denotes that the object will
    /// be mutated but not moved.
    pub fn compute_slider_cursor_pos(
        mut h: Pin<&mut OsuObject>,
        radius: f64,
    ) -> Pin<&mut OsuObject> {
        let pos = h.pos;
        let stack_offset = h.stack_offset;

        let OsuObjectKind::Slider(ref mut slider) = h.kind else {
            return h;
        };

        let mut curr_cursor_pos = pos + stack_offset;
        let scaling_factor = OsuDifficultyObject::NORMALIZED_RADIUS / radius;

        for (curr_movement_obj, i) in slider.nested_objects.iter().zip(1..) {
            let mut curr_movement = curr_movement_obj.pos + stack_offset - curr_cursor_pos;
            let mut curr_movement_len = scaling_factor * f64::from(curr_movement.length());
            let mut required_movement = OsuDifficultyObject::ASSUMED_SLIDER_RADIUS;

            if i == slider.nested_objects.len() {
                let lazy_movement = slider.lazy_end_pos - curr_cursor_pos;

                if lazy_movement.length() < curr_movement.length() {
                    curr_movement = lazy_movement;
                }

                curr_movement_len = scaling_factor * f64::from(curr_movement.length());
            } else if curr_movement_obj.is_repeat() {
                required_movement = OsuDifficultyObject::NORMALIZED_RADIUS;
            }

            if curr_movement_len > required_movement {
                curr_cursor_pos += curr_movement
                    * ((curr_movement_len - required_movement) / curr_movement_len) as f32;
                curr_movement_len *= (curr_movement_len - required_movement) / curr_movement_len;
                slider.lazy_travel_dist += curr_movement_len as f32;
            }

            
            
            if i == slider.nested_objects.len() {
                slider.lazy_end_pos = curr_cursor_pos;
            }
        }

        h
    }

    fn get_end_cursor_pos(hit_object: &OsuObject) -> Pos {
        if let OsuObjectKind::Slider(ref slider) = hit_object.kind {
            // We don't have access to the slider's curve at this point so we
            // take the pre-computed value.
            slider.lazy_end_pos
        } else {
            hit_object.stacked_pos()
        }
    }
}

impl IDifficultyObject for OsuDifficultyObject<'_> {
    fn idx(&self) -> usize {
        self.idx
    }
}
