use std::{borrow::Cow, sync::OnceLock};

use crate::{Correctness, DICTIONARY, Guess, Guesser};

static INITIAL: OnceLock<Vec<(&'static str, usize)>> = OnceLock::new();

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
    pub rank_by: Rank,
    pub sigmoid: bool,
    pub cut_off: bool,
}

impl Default for SolverBuilder {
    fn default() -> Self {
        Self {
            rank_by: Rank::ExpectedScore,
            sigmoid: true,
            cut_off: true,
        }
    }
}

impl SolverBuilder {
    pub fn build(self) -> Solver {
        let candidates = Cow::Borrowed(INITIAL.get_or_init(|| {
            let mut words = DICTIONARY;

            if self.sigmoid {
                let total_words = words.len() as f64;
                let cut_off_rank = 1.0 - N_COMMON_WORDS / total_words;

                for (rank, (_, frequency)) in words.iter_mut().enumerate() {
                    let x = 1.0 - rank as f64 / total_words;
                    let z = SIGMOID_STEEPNESS * (cut_off_rank - x);
                    let sigmoid = 1.0 / (1.0 + z.exp());

                    *frequency = (sigmoid * WEIGHT_SCALE).round() as usize + 1;
                }
            }

            words.to_vec()
        }));

        Solver {
            candidates,
            options: self,
        }
    }
}

pub struct Solver {
    candidates: Cow<'static, Vec<(&'static str, usize)>>,
    options: SolverBuilder,
}

impl Solver {
    const FIRST_GUESS: &str = "tares";

    pub fn builder() -> SolverBuilder {
        SolverBuilder::default()
    }

    fn retain(&mut self, mut f: impl FnMut(&str, usize) -> bool) {
        if let Cow::Owned(ref mut candidates) = self.candidates {
            candidates.retain(|&(word, frequency)| f(word, frequency));
        } else {
            self.candidates = Cow::Owned(
                self.candidates
                    .iter()
                    .filter(|&(word, frequency)| f(word, *frequency))
                    .copied()
                    .collect(),
            );
        }
    }
}

impl Guesser for Solver {
    fn guess(&mut self, history: &[Guess]) -> String {
        if history.is_empty() {
            return Self::FIRST_GUESS.to_string();
        }

        if let Some(last_guess) = history.last() {
            self.retain(|word, _| last_guess.allows(word));
        }

        let mut best_word = self.candidates[0].0;
        let mut min_loss = f64::INFINITY;

        if self.candidates.len() > 2 {
            let (total_weight, max_possible_sum, prior_entropy) = match self.options.rank_by {
                Rank::WeightedInformation => {
                    let total = self
                        .candidates
                        .iter()
                        .map(|&(_, frequency)| frequency)
                        .sum::<usize>() as f64;
                    (total, total * total.log2(), 0.0)
                }
                Rank::ExpectedScore => {
                    let total = self
                        .candidates
                        .iter()
                        .map(|&(_, weight)| weight)
                        .sum::<usize>() as f64;
                    let prior_entropy = -self
                        .candidates
                        .iter()
                        .map(|&(_, weight)| {
                            let p = weight as f64 / total;
                            p * p.log2()
                        })
                        .sum::<f64>();
                    (total, total * total.log2(), prior_entropy)
                }
                _ => (0.0, 0.0, 0.0),
            };

            let guesses_made = history.len() as f64;
            let cut_off = if self.options.cut_off {
                (self.candidates.len() / 3).max(20)
            } else {
                usize::MAX
            };

            for &(guess_word, guess_frequency) in self.candidates.iter().take(cut_off) {
                let guess_frequency = guess_frequency as f64;
                let max_allowed_sum = match self.options.rank_by {
                    Rank::ExpectedInformation => min_loss,
                    Rank::InfoPlusProbability => min_loss + guess_frequency,
                    Rank::WeightedInformation => {
                        if min_loss.is_finite() {
                            max_possible_sum + min_loss / guess_frequency
                        } else {
                            f64::INFINITY
                        }
                    }
                    Rank::ExpectedScore => {
                        let p_correct = guess_frequency / total_weight;
                        let p_miss = 1.0 - p_correct;

                        if min_loss.is_finite() && p_miss > 0.0 {
                            let max_estimated_guesses =
                                (min_loss - guesses_made - p_correct) / p_miss;
                            let max_remaining_entropy =
                                (max_estimated_guesses.exp() - 3.679) / 3.870;
                            if max_remaining_entropy <= 0.0 {
                                continue;
                            }
                            let min_expected_info = prior_entropy - max_remaining_entropy;
                            max_possible_sum - (min_expected_info * total_weight)
                        } else {
                            f64::INFINITY
                        }
                    }
                };

                if max_allowed_sum <= 0.0 {
                    continue;
                }

                let mut pattern_weights = [0; 243];
                for &(target_word, target_frequency) in self.candidates.iter() {
                    let mask = Correctness::compute(target_word, guess_word);
                    unsafe {
                        *pattern_weights.get_unchecked_mut(mask_to_index(mask)) += target_frequency;
                    }
                }

                let sum = pattern_weights
                    .into_iter()
                    .filter(|&weight| weight > 1)
                    .try_fold(0.0, |acc, weight| {
                        let w = weight as f64;
                        let next = acc + w * w.log2();
                        if next < max_allowed_sum {
                            Ok(next)
                        } else {
                            Err(next)
                        }
                    })
                    .unwrap_or_else(|sum| sum);

                if sum < max_allowed_sum {
                    min_loss = match self.options.rank_by {
                        Rank::ExpectedInformation => sum,
                        Rank::InfoPlusProbability => sum - guess_frequency,
                        Rank::WeightedInformation => -guess_frequency * (max_possible_sum - sum),
                        Rank::ExpectedScore => {
                            let p_correct = guess_frequency / total_weight;
                            let p_miss = 1.0 - p_correct;
                            let expected_info = (max_possible_sum - sum) / total_weight;
                            let remaining_entropy = (prior_entropy - expected_info).max(0.0);
                            let estimated_guesses = estimate_remaining_guesses(remaining_entropy);

                            p_correct * (guesses_made + 1.0)
                                + p_miss * (guesses_made + estimated_guesses)
                        }
                    };

                    best_word = guess_word;
                }
            }
        }

        best_word.to_string()
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

#[inline(always)]
fn estimate_remaining_guesses(remaining_entropy: f64) -> f64 {
    (remaining_entropy * 3.870 + 3.679).ln()
}
