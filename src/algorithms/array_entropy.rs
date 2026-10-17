use std::{borrow::Cow, sync::OnceLock};

use crate::{Correctness, DICTIONARY, Guess, Guesser};

static INITIAL: OnceLock<Vec<(&'static str, usize)>> = OnceLock::new();

pub struct ArrayEntropy {
    candidates: Cow<'static, [(&'static str, usize)]>,
}

impl ArrayEntropy {
    const FIRST_GUESS: &str = "tares";

    pub fn new() -> Self {
        Self {
            candidates: Cow::Borrowed(INITIAL.get_or_init(|| DICTIONARY.to_vec())),
        }
    }
}

impl Guesser for ArrayEntropy {
    fn guess(&mut self, history: &[Guess]) -> String {
        if history.is_empty() {
            return Self::FIRST_GUESS.to_string();
        }

        if let Some(last_guess) = history.last() {
            if let Cow::Owned(ref mut candidates) = self.candidates {
                candidates.retain(|(word, _)| last_guess.allows(word));
            } else {
                self.candidates = Cow::Owned(
                    self.candidates
                        .iter()
                        .filter(|(word, _)| last_guess.allows(word))
                        .copied()
                        .collect(),
                );
            }
        }

        let mut best_word = self.candidates[0].0;
        let mut min_score = 0.0;

        if self.candidates.len() > 2 {
            for &(guess_word, _) in self.candidates.iter() {
                let mut pattern_weights = [0; 243];

                for &(target_word, frequency) in self.candidates.iter() {
                    let mask = Correctness::compute(target_word, guess_word);
                    pattern_weights[mask_to_index(mask)] += frequency;
                }

                let mut score = 0.0;
                for weight in pattern_weights {
                    if weight > 1 {
                        let weight = weight as f64;
                        score += weight * weight.log2();
                        if score >= min_score {
                            break;
                        }
                    }
                }

                if score < min_score {
                    min_score = score;
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
