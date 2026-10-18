use std::{borrow::Cow, sync::OnceLock};

use crate::{Correctness, DICTIONARY, Guess, Guesser};

static SIGMOID_INITIAL: OnceLock<Vec<(&'static str, usize)>> = OnceLock::new();

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

    pub fn build(self) -> Solver {
        let candidates = Cow::Borrowed(if self.sigmoid {
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
        });

        Solver {
            candidates,
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

    fn retain(&mut self, mut f: impl FnMut(&str, usize) -> bool) {
        if let Cow::Owned(ref mut candidates) = self.candidates {
            candidates.retain(|&(word, weight)| f(word, weight));
        } else {
            self.candidates = Cow::Owned(
                self.candidates
                    .iter()
                    .filter(|&&(word, weight)| f(word, weight))
                    .copied()
                    .collect(),
            );
        }
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
        match rank {
            Rank::WeightedInformation => {
                let total = candidates.iter().map(|&(_, weight)| weight).sum::<usize>() as f64;
                Self {
                    rank,
                    total_weight: total,
                    max_possible_sum: total * total.log2(),
                    baseline_sum: 0.0,
                }
            }
            Rank::ExpectedScore => {
                let (total, baseline_sum) =
                    candidates
                        .iter()
                        .fold((0.0, 0.0), |(acc_total, acc_sum), &(_, weight)| {
                            let weight = weight as f64;
                            let new_total = acc_total + weight;
                            let new_sum = if weight > 1.0 {
                                acc_sum + weight * weight.log2()
                            } else {
                                acc_sum
                            };
                            (new_total, new_sum)
                        });

                Self {
                    rank,
                    total_weight: total,
                    max_possible_sum: total * total.log2(),
                    baseline_sum,
                }
            }
            _ => Self {
                rank,
                total_weight: 0.0,
                max_possible_sum: 0.0,
                baseline_sum: 0.0,
            },
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
                let p_miss = 1.0 - p_correct;
                let remaining_entropy = ((sum - self.baseline_sum) / self.total_weight).max(0.0);
                p_correct + p_miss * estimate_remaining_guesses(remaining_entropy)
            }
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

        if self.candidates.len() > 2 {
            let context = ScoreContext::new(self.options.rank, &self.candidates);
            let mut min_loss = f64::INFINITY;

            let cut_off = if self.options.cut_off {
                (self.candidates.len() / 3).max(20)
            } else {
                usize::MAX
            };

            for &(guess_word, guess_weight) in self.candidates.iter().take(cut_off) {
                let guess_weight = guess_weight as f64;
                let max_allowed_sum = context.max_allowed_sum(min_loss, guess_weight);
                if max_allowed_sum <= 0.0 {
                    continue;
                }

                let mut pattern_weights = [0; 243];
                for &(target_word, target_weight) in self.candidates.iter() {
                    let mask = Correctness::compute(target_word, guess_word);
                    unsafe {
                        *pattern_weights.get_unchecked_mut(mask_to_index(mask)) += target_weight;
                    }
                }

                let sum = pattern_weights
                    .into_iter()
                    .filter(|&weight| weight > 1)
                    .try_fold(0.0, |acc, weight| {
                        let weight = weight as f64;
                        let next = acc + weight * weight.log2();
                        if next < max_allowed_sum {
                            Ok(next)
                        } else {
                            Err(next)
                        }
                    })
                    .unwrap_or_else(|sum| sum);

                if sum < max_allowed_sum {
                    min_loss = context.compute_loss(sum, guess_weight);
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
