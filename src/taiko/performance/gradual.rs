use crate::{
    taiko::{difficulty::gradual::TaikoGradualDifficulty, TaikoBeatmap, TaikoScoreState},
    ModeDifficulty,
};

use super::TaikoPerformanceAttributes;

/// Gradually calculate the performance attributes of an osu!taiko map.
///
/// After each hit object you can call [`next`] and it will return the
/// resulting current [`TaikoPerformanceAttributes`]. To process multiple
/// objects at once, use [`nth`] instead.
///
/// Both methods require a [`TaikoScoreState`] that contains the current
/// hitresults as well as the maximum combo so far.
///
/// If you only want to calculate difficulty attributes use
/// [`TaikoGradualDifficulty`] instead.
///
/// # Example
///
/// ```
/// use rosu_pp::{Beatmap, ModeDifficulty};
/// use rosu_pp::taiko::{Taiko, TaikoGradualPerformance, TaikoScoreState};
///
/// let converted = Beatmap::from_path("./resources/1028484.osu")
///     .unwrap()
///     .unchecked_into_converted::<Taiko>();
///
/// let difficulty = ModeDifficulty::new().mods(64); // DT
/// let mut gradual_perf = TaikoGradualPerformance::new(&difficulty, &converted);
/// let mut state = TaikoScoreState::new(); // empty state, everything is on 0.
///
/// // The first 10 hitresults are 300s
/// for _ in 0..10 {
///     state.n300 += 1;
///     state.max_combo += 1;
///
///     let performance = gradual_perf.next(state.clone()).unwrap();
///     println!("PP: {}", performance.pp);
/// }
///
/// // Then comes a miss.
/// // Note that state's max combo won't be incremented for
/// // the next few objects because the combo is reset.
/// state.misses += 1;
/// let performance = gradual_perf.next(state.clone()).unwrap();
/// println!("PP: {}", performance.pp);
///
/// // The next 10 objects will be a mixture of 300s and 100s.
/// // Notice how all 10 objects will be processed in one go.
/// state.n300 += 3;
/// state.n100 += 7;
/// // The `nth` method takes a zero-based value.
/// let performance = gradual_perf.nth(state.clone(), 9).unwrap();
/// println!("PP: {}", performance.pp);
///
/// // Now comes another 300. Note that the max combo gets incremented again.
/// state.n300 += 1;
/// state.max_combo += 1;
/// let performance = gradual_perf.next(state.clone()).unwrap();
/// println!("PP: {}", performance.pp);
///
/// // Skip to the end
/// # /*
/// state.max_combo = ...
/// state.n300 = ...
/// state.n100 = ...
/// state.misses = ...
/// # */
/// let final_performance = gradual_perf.nth(state.clone(), usize::MAX).unwrap();
/// println!("PP: {}", performance.pp);
///
/// // Once the final performance was calculated,
/// // attempting to process further objects will return `None`.
/// assert!(gradual_perf.next(state).is_none());
/// ```
///
/// [`next`]: TaikoGradualPerformance::next
/// [`nth`]: TaikoGradualPerformance::nth
pub struct TaikoGradualPerformance {
    difficulty: TaikoGradualDifficulty,
}

impl TaikoGradualPerformance {
    /// Create a new gradual performance calculator for osu!taiko maps.
    pub fn new(difficulty: &ModeDifficulty, converted: &TaikoBeatmap<'_>) -> Self {
        let difficulty = TaikoGradualDifficulty::new(difficulty, converted);

        Self { difficulty }
    }

    /// Process the next hit object and calculate the performance attributes
    /// for the resulting score.
    pub fn next(&mut self, state: TaikoScoreState) -> Option<TaikoPerformanceAttributes> {
        self.nth(state, 0)
    }

    /// Process all remaining hit objects and calculate the final performance
    /// attributes.
    pub fn last(&mut self, state: TaikoScoreState) -> Option<TaikoPerformanceAttributes> {
        self.nth(state, usize::MAX)
    }

    /// Process everything up the the next `n`th hit object and calculate the
    /// performance attributes for the resulting score state.
    ///
    /// Note that the count is zero-indexed, so `n=0` will process 1 object,
    /// `n=1` will process 2, and so on.
    pub fn nth(&mut self, state: TaikoScoreState, n: usize) -> Option<TaikoPerformanceAttributes> {
        let performance = self
            .difficulty
            .nth(n)?
            .performance()
            .state(state)
            .mods(self.difficulty.mods)
            .clock_rate(self.difficulty.clock_rate)
            .passed_objects(self.difficulty.idx as u32)
            .calculate();

        Some(performance)
    }
}

#[cfg(test)]
mod tests {
    use crate::{taiko::TaikoPerformance, Beatmap};

    use super::*;

    #[test]
    fn next_and_nth() {
        let converted = Beatmap::from_path("./resources/1028484.osu")
            .unwrap()
            .unchecked_into_converted();

        let mods = 88; // HDHRDT
        let difficulty = ModeDifficulty::new().mods(88);

        let mut gradual = TaikoGradualPerformance::new(&difficulty, &converted);
        let mut gradual_2nd = TaikoGradualPerformance::new(&difficulty, &converted);
        let mut gradual_3rd = TaikoGradualPerformance::new(&difficulty, &converted);

        let mut state = TaikoScoreState::default();

        let hit_objects_len = converted.map.hit_objects.len();

        let n_hits = converted
            .map
            .hit_objects
            .iter()
            .filter(|h| h.is_circle())
            .count();

        for i in 1.. {
            state.misses += 1;

            let Some(next_gradual) = gradual.next(state) else {
                assert_eq!(i, n_hits + 1);
                assert!(gradual_2nd.last(state).is_some() || hit_objects_len % 2 == 0);
                assert!(gradual_3rd.last(state).is_some() || hit_objects_len % 3 == 0);
                break;
            };

            if i % 2 == 0 {
                let next_gradual_2nd = gradual_2nd.nth(state, 1).unwrap();
                assert_eq!(next_gradual, next_gradual_2nd);
            }

            if i % 3 == 0 {
                let next_gradual_3rd = gradual_3rd.nth(state, 2).unwrap();
                assert_eq!(next_gradual, next_gradual_3rd);
            }

            let mut regular_calc = TaikoPerformance::new(converted.as_owned())
                .mods(mods)
                .passed_objects(i as u32)
                .state(state);

            let regular_state = regular_calc.generate_state();
            assert_eq!(state, regular_state);

            let expected = regular_calc.calculate();

            assert_eq!(next_gradual, expected);
        }
    }
}
