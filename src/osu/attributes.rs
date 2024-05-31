use crate::osu::performance::OsuPerformance;

/// The result of a difficulty calculation on an osu!standard map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OsuDifficultyAttributes {
    /// The difficulty of the aim skill.
    pub aim: f64,
    pub jump: f64,
    pub flow: f64,
    pub precision: f64,
    pub speed: f64,
    pub stamina: f64,
    pub accuracy: f64,
    /// The approach rate.
    pub ar: f64,
    /// The overall difficulty
    pub od: f64,
    /// The health drain rate.
    pub hp: f64,
    /// The amount of circles.
    pub n_circles: u32,
    /// The amount of sliders.
    pub n_sliders: u32,
    /// The amount of spinners.
    pub n_spinners: u32,
    /// The final star rating
    pub stars: f64,
    /// The maximum combo.
    pub max_combo: u32,
}

impl OsuDifficultyAttributes {
    /// Return the maximum combo.
    pub const fn max_combo(&self) -> u32 {
        self.max_combo
    }

    /// Return the amount of hitobjects.
    pub const fn n_objects(&self) -> u32 {
        self.n_circles + self.n_sliders + self.n_spinners
    }

    /// Returns a builder for performance calculation.
    pub fn performance<'a>(self) -> OsuPerformance<'a> {
        self.into()
    }
}

/// The result of a performance calculation on an osu!standard map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OsuPerformanceAttributes {
    /// The difficulty attributes that were used for the performance calculation
    pub difficulty: OsuDifficultyAttributes,
    /// The final performance points.
    pub pp: f64,
    pub pp_aim: f64,
    pub pp_jump_aim: f64,
    pub pp_flow_aim: f64,
    pub pp_precision: f64,
    pub pp_speed: f64,
    pub pp_stamina: f64,
    pub pp_accuracy: f64,
}

impl OsuPerformanceAttributes {
    /// Return the star value.
    pub const fn stars(&self) -> f64 {
        self.difficulty.stars
    }

    /// Return the performance point value.
    pub const fn pp(&self) -> f64 {
        self.pp
    }

    /// Return the maximum combo of the map.
    pub const fn max_combo(&self) -> u32 {
        self.difficulty.max_combo
    }
    /// Return the amount of hitobjects.
    pub const fn n_objects(&self) -> u32 {
        self.difficulty.n_objects()
    }

    /// Returns a builder for performance calculation.
    pub fn performance<'a>(self) -> OsuPerformance<'a> {
        self.difficulty.into()
    }
}

impl From<OsuPerformanceAttributes> for OsuDifficultyAttributes {
    fn from(attributes: OsuPerformanceAttributes) -> Self {
        attributes.difficulty
    }
}
