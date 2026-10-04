//! The match score: how well an account fits a campaign's sliders and genres. It ranks a
//! creator's feed and, in winner selection, weighs against a bid's price.

use serde::Serialize;

/// Engagement at or above this rate counts in full.
const FULL_ENGAGEMENT_RATE: f64 = 0.10;
/// Covering the campaign's genres weighs as much as a slider at its midpoint.
const GENRE_WEIGHT: f64 = 0.5;

#[derive(Debug, Clone, Copy)]
pub struct Account<'a> {
    /// Interactions per view, 0 to 1.
    pub engagement_rate: f64,
    /// 0 to 100.
    pub quality_score: i16,
    /// The creator's, 0 to 100.
    pub reliability_score: i16,
    pub genre_ids: &'a [i16],
}

/// A campaign's sliders, each 0 to 100, and the genres it looks for.
#[derive(Debug, Clone, Copy)]
pub struct Preferences<'a> {
    pub engagement_weight: i16,
    pub quality_weight: i16,
    pub reliability_weight: i16,
    /// Empty for any genre.
    pub genre_ids: &'a [i16],
}

/// How well an account fits a campaign, and why.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Match {
    /// 0 to 100.
    pub score: i16,
    /// Largest contribution first, without the factors the campaign gives no weight.
    pub factors: Vec<Factor>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Factor {
    pub kind: FactorKind,
    /// How much the campaign cares about it, 0 to 1.
    pub weight: f64,
    /// How well the account does on it, 0 to 1.
    pub value: f64,
}

impl Factor {
    fn contribution(self) -> f64 {
        self.weight * self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FactorKind {
    Engagement,
    Quality,
    Reliability,
    Genre,
}

/// The factors' mean, weighted by how much the campaign cares about each. Factors are on a
/// fixed scale, not relative to other accounts, so a score does not shift as accounts come and go.
#[must_use]
pub fn score(account: &Account, preferences: &Preferences) -> Match {
    let mut factors = vec![
        Factor {
            kind: FactorKind::Engagement,
            weight: slider_weight(preferences.engagement_weight),
            value: (account.engagement_rate / FULL_ENGAGEMENT_RATE).min(1.0),
        },
        Factor {
            kind: FactorKind::Quality,
            weight: slider_weight(preferences.quality_weight),
            value: f64::from(account.quality_score) / 100.0,
        },
        Factor {
            kind: FactorKind::Reliability,
            weight: slider_weight(preferences.reliability_weight),
            value: f64::from(account.reliability_score) / 100.0,
        },
    ];
    if !preferences.genre_ids.is_empty() {
        let covered = preferences
            .genre_ids
            .iter()
            .filter(|genre| account.genre_ids.contains(genre))
            .count();
        #[expect(
            clippy::cast_precision_loss,
            reason = "a campaign has at most 21 genres"
        )]
        let value = covered as f64 / preferences.genre_ids.len() as f64;
        factors.push(Factor {
            kind: FactorKind::Genre,
            weight: GENRE_WEIGHT,
            value,
        });
    }
    factors.retain(|factor| factor.weight > 0.0);
    if factors.is_empty() {
        // The campaign prefers nothing, so every eligible account fits it equally well.
        return Match {
            score: 100,
            factors,
        };
    }

    let total_weight: f64 = factors.iter().map(|factor| factor.weight).sum();
    let total: f64 = factors.iter().map(|factor| factor.contribution()).sum();
    #[expect(
        clippy::cast_possible_truncation,
        reason = "a weighted mean of values from 0 to 1, times 100"
    )]
    let score = (100.0 * total / total_weight).round() as i16;
    factors.sort_by(|a, b| b.contribution().total_cmp(&a.contribution()));
    Match { score, factors }
}

fn slider_weight(slider: i16) -> f64 {
    f64::from(slider) / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;

    const LENA: Account = Account {
        engagement_rate: 0.12,
        quality_score: 72,
        reliability_score: 88,
        genre_ids: &[5, 14],
    };

    const INDIFFERENT: Preferences = Preferences {
        engagement_weight: 0,
        quality_weight: 0,
        reliability_weight: 0,
        genre_ids: &[],
    };

    fn kinds(m: &Match) -> Vec<FactorKind> {
        m.factors.iter().map(|factor| factor.kind).collect()
    }

    #[test]
    fn no_preference_scores_every_account_100() {
        let m = score(&LENA, &INDIFFERENT);

        assert_eq!(m.score, 100);
        assert!(m.factors.is_empty());
    }

    #[test]
    fn each_slider_alone_scores_its_measure() {
        let engagement = Preferences {
            engagement_weight: 100,
            ..INDIFFERENT
        };
        let quality = Preferences {
            quality_weight: 100,
            ..INDIFFERENT
        };
        let reliability = Preferences {
            reliability_weight: 100,
            ..INDIFFERENT
        };

        // Engagement above 10% counts in full.
        assert_eq!(score(&LENA, &engagement).score, 100);
        assert_eq!(score(&LENA, &quality).score, 72);
        assert_eq!(score(&LENA, &reliability).score, 88);
        let half_engaged = Account {
            engagement_rate: 0.05,
            ..LENA
        };
        assert_eq!(score(&half_engaged, &engagement).score, 50);
    }

    #[test]
    fn sliders_weigh_their_factors_and_order_them_by_contribution() {
        let preferences = Preferences {
            engagement_weight: 25,
            quality_weight: 100,
            reliability_weight: 25,
            genre_ids: &[],
        };

        let m = score(&LENA, &preferences);

        // (0.25 × 1 + 1 × 0.72 + 0.25 × 0.88) / (0.25 + 1 + 0.25) = 0.79
        assert_eq!(m.score, 79);
        assert_eq!(
            kinds(&m),
            [
                FactorKind::Quality,
                FactorKind::Engagement,
                FactorKind::Reliability
            ]
        );
        assert_eq!(
            m.factors[2],
            Factor {
                kind: FactorKind::Reliability,
                weight: 0.25,
                value: 0.88,
            }
        );
    }

    #[test]
    fn genre_factor_is_the_share_of_the_campaigns_genres_covered() {
        let preferences = Preferences {
            genre_ids: &[1, 5, 9, 14],
            ..INDIFFERENT
        };

        let m = score(&LENA, &preferences);

        assert_eq!(m.score, 50);
        assert_eq!(
            m.factors,
            [Factor {
                kind: FactorKind::Genre,
                weight: 0.5,
                value: 0.5,
            }]
        );
        let none_covered = Account {
            genre_ids: &[],
            ..LENA
        };
        assert_eq!(score(&none_covered, &preferences).score, 0);
    }
}
