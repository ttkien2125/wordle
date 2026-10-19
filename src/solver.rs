use std::{borrow::Cow, sync::OnceLock};

use crate::{Correctness, DICTIONARY, Guess, Guesser};

static SIGMOID_INITIAL: OnceLock<Vec<(&'static str, usize)>> = OnceLock::new();
static TURN2_CACHE: OnceLock<[&'static str; 243]> = OnceLock::new();

const N_COMMON_WORDS: f64 = 3000.0;
const SIGMOID_STEEPNESS: f64 = 10.0;
const WEIGHT_SCALE: f64 = 1_000_000.0;

#[derive(Clone, Copy, PartialEq)]
pub enum Rank {
    /// Pick based on E[information]
    ExpectedInformation,
    /// Pick based on p(word) + E[information]
    InfoPlusProbability,
    /// Pick based on p(word) * E[information]
    WeightedInformation,
    /// Pick based on E[score] = p(word) * (score + 1) + (1 - p(word)) * (score + E[guesses](entropy - E[information]))
    ExpectedScore,
}

#[derive(Clone, Copy)]
pub struct SolverBuilder {
    pub rank: Rank,
    pub sigmoid: bool,
    pub cut_off: bool,
}

impl Default for SolverBuilder {
    fn default() -> Self {
        Self {
            rank: Rank::ExpectedScore,
            sigmoid: true,
            cut_off: true,
        }
    }
}

impl SolverBuilder {
    pub fn rank(mut self, rank: Rank) -> Self {
        self.rank = rank;
        self
    }

    pub fn sigmoid(mut self, sigmoid: bool) -> Self {
        self.sigmoid = sigmoid;
        self
    }

    pub fn cut_off(mut self, cut_off: bool) -> Self {
        self.cut_off = cut_off;
        self
    }

    fn initial_candidates(self) -> &'static [(&'static str, usize)] {
        if self.sigmoid {
            SIGMOID_INITIAL
                .get_or_init(|| {
                    let total_words = DICTIONARY.len() as f64;
                    DICTIONARY
                        .iter()
                        .enumerate()
                        .map(|(rank, &(word, _))| {
                            let z =
                                (rank as f64 - N_COMMON_WORDS) * SIGMOID_STEEPNESS / total_words;
                            let sigmoid = 1.0 / (1.0 + z.exp());

                            let weight = (sigmoid * WEIGHT_SCALE).round() as usize + 1;
                            (word, weight)
                        })
                        .collect()
                })
                .as_slice()
        } else {
            &DICTIONARY[..]
        }
    }

    pub fn build(self) -> Solver {
        Solver {
            candidates: Cow::Borrowed(self.initial_candidates()),
            options: self,
        }
    }
}

pub struct Solver {
    candidates: Cow<'static, [(&'static str, usize)]>,
    options: SolverBuilder,
}

impl Solver {
    const FIRST_GUESS: &str = "tares";

    pub fn builder() -> SolverBuilder {
        SolverBuilder::default()
    }

    fn init_turn2_cache(&self) -> [&'static str; 243] {
        let mut buckets: [Vec<(&'static str, usize)>; 243] = std::array::from_fn(|_| Vec::new());
        for &(word, weight) in self.options.initial_candidates() {
            let mask = Correctness::compute(word, Solver::FIRST_GUESS);
            buckets[mask_to_index(mask)].push((word, weight));
        }

        std::array::from_fn(|index| self.pick_best(&buckets[index]))
    }

    fn pick_best(&self, candidates: &[(&'static str, usize)]) -> &'static str {
        if candidates.len() <= 2 {
            return candidates
                .first()
                .map_or(Self::FIRST_GUESS, |&(word, _)| word);
        }

        let context = ScoreContext::new(self.options.rank, candidates);
        let mut min_loss = f64::INFINITY;
        let mut best_word = candidates[0].0;

        let cut_off = if self.options.cut_off {
            (candidates.len() / 3).max(20)
        } else {
            usize::MAX
        };

        for &(guess_word, guess_weight) in candidates.iter().take(cut_off) {
            let guess_weight = guess_weight as f64;
            let max_allowed_sum = context.max_allowed_sum(min_loss, guess_weight);
            if max_allowed_sum <= 0.0 {
                continue;
            }

            let mut pattern_weights = [0; 243];
            for &(target_word, target_weight) in candidates.iter() {
                let mask = Correctness::compute(target_word, guess_word);
                unsafe {
                    *pattern_weights.get_unchecked_mut(mask_to_index(mask)) += target_weight;
                }
            }

            let mut sum = 0.0;
            for weight in pattern_weights {
                if weight > 1 {
                    let weight = weight as f64;
                    sum += weight * weight.log2();
                    if sum >= max_allowed_sum {
                        break;
                    }
                }
            }

            if sum < max_allowed_sum {
                min_loss = context.compute_loss(sum, guess_weight);
                best_word = guess_word;
            }
        }

        best_word
    }
}

struct ScoreContext {
    rank: Rank,
    total_weight: f64,
    max_possible_sum: f64,
    baseline_sum: f64,
}

impl ScoreContext {
    fn new(rank: Rank, candidates: &[(&str, usize)]) -> Self {
        let mut total_weight = 0.0;
        let mut baseline_sum = 0.0;

        if matches!(rank, Rank::WeightedInformation | Rank::ExpectedScore) {
            for &(_, weight) in candidates {
                let weight = weight as f64;
                total_weight += weight;
                if rank == Rank::ExpectedScore && weight > 1.0 {
                    baseline_sum += weight * weight.log2();
                }
            }
        }

        Self {
            rank,
            total_weight,
            max_possible_sum: if total_weight > 0.0 {
                total_weight * total_weight.log2()
            } else {
                0.0
            },
            baseline_sum,
        }
    }

    fn max_allowed_sum(&self, min_loss: f64, guess_weight: f64) -> f64 {
        match self.rank {
            Rank::ExpectedInformation => min_loss,
            Rank::InfoPlusProbability => min_loss + guess_weight,
            Rank::WeightedInformation => self.max_possible_sum + min_loss / guess_weight,
            Rank::ExpectedScore => {
                let p_correct = guess_weight / self.total_weight;
                let p_miss = 1.0 - p_correct;

                if p_miss <= 0.0 {
                    return f64::INFINITY;
                }

                let max_estimated_guesses = (min_loss - p_correct) / p_miss;
                let max_remaining_entropy = (max_estimated_guesses.exp() - 3.679) / 3.870;
                if max_remaining_entropy <= 0.0 {
                    return 0.0;
                }

                self.baseline_sum + max_remaining_entropy * self.total_weight
            }
        }
    }

    fn compute_loss(&self, sum: f64, guess_weight: f64) -> f64 {
        match self.rank {
            Rank::ExpectedInformation => sum,
            Rank::InfoPlusProbability => sum - guess_weight,
            Rank::WeightedInformation => guess_weight * (sum - self.max_possible_sum),
            Rank::ExpectedScore => {
                let p_correct = guess_weight / self.total_weight;
                let remaining_entropy = ((sum - self.baseline_sum) / self.total_weight).max(0.0);
                let remaining_guesses = (remaining_entropy * 3.870 + 3.679).ln();
                p_correct + (1.0 - p_correct) * remaining_guesses
            }
        }
    }
}

impl Guesser for Solver {
    fn guess(&mut self, history: &[Guess]) -> String {
        let Some(last_guess) = history.last() else {
            return Self::FIRST_GUESS.to_string();
        };

        if history.len() == 1 {
            let cache = TURN2_CACHE.get_or_init(|| self.init_turn2_cache());
            let index = mask_to_index(last_guess.mask);
            return cache[index].to_string();
        }

        match self.candidates {
            Cow::Borrowed(candidates) => {
                self.candidates = Cow::Owned(
                    candidates
                        .iter()
                        .copied()
                        .filter(|&(word, _)| history.iter().all(|guess| guess.allows(word)))
                        .collect(),
                );
            }
            Cow::Owned(ref mut candidates) => {
                candidates.retain(|&(word, _)| last_guess.allows(word));
            }
        }

        self.pick_best(&self.candidates).to_string()
    }
}

#[inline(always)]
fn mask_to_index(mask: [Correctness; 5]) -> usize {
    mask.iter().fold(0, |acc, correctness| {
        let digit = match correctness {
            Correctness::Correct => 0,
            Correctness::Misplaced => 1,
            Correctness::Wrong => 2,
        };
        acc * 3 + digit
    })
}
