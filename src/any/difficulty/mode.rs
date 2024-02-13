use crate::{
    model::{beatmap::Converted, mode::IGameMode},
    util::mods::Mods,
};

/// Difficulty calculator on maps of a given mode.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModeDifficulty {
    mods: u32,
    passed_objects: Option<u32>,
    clock_rate: Option<f64>,
}

impl ModeDifficulty {
    /// Create a new difficulty calculator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Specify mods through their bit values.
    ///
    /// See [https://github.com/ppy/osu-api/wiki#mods](https://github.com/ppy/osu-api/wiki#mods)
    pub fn mods(&mut self, mods: u32) -> &mut Self {
        self.mods = mods;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    pub fn passed_objects(&mut self, passed_objects: u32) -> &mut Self {
        self.passed_objects = Some(passed_objects);

        self
    }

    /// Adjust the clock rate used in the calculation.
    ///
    /// If none is specified, it will take the clock rate based on the mods
    /// i.e. 1.5 for DT, 0.75 for HT and 1.0 otherwise.
    pub fn clock_rate(&mut self, clock_rate: f64) -> &mut Self {
        self.clock_rate = Some(clock_rate);

        self
    }

    /// Perform the difficulty calculation for a [`Converted`] beatmap and
    /// process the final skill values.
    pub fn calculate<M: IGameMode>(&self, map: &Converted<'_, M>) -> M::DifficultyAttributes {
        M::difficulty(self, map)
    }

    /// Perform a difficulty calculation for a [`Converted`] beatmap without
    /// processing the final skill values.
    pub fn strains<M: IGameMode>(&self, map: &Converted<'_, M>) -> M::Strains {
        M::strains(self, map)
    }

    pub(crate) const fn get_mods(&self) -> u32 {
        self.mods
    }

    pub(crate) fn get_clock_rate(&self) -> f64 {
        self.clock_rate.unwrap_or_else(|| self.mods.clock_rate())
    }

    pub(crate) fn get_passed_objects(&self) -> usize {
        self.passed_objects.map_or(usize::MAX, |n| n as usize)
    }
}