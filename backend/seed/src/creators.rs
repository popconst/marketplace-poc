//! Fake creators and their platform accounts. The constants are our assumptions about WePush's
//! creators, not measurements; weights are relative.

use std::ops::RangeInclusive;

use chrono::{DateTime, TimeDelta, Utc};
use db::models::{Creator, Platform, PlatformAccount};
use deunicode::deunicode;
use fake::locales::{DE_DE, Data, EN};
use rand::distr::Distribution;
use rand::distr::weighted::WeightedIndex;
use rand::rngs::ChaCha8Rng;
use rand::seq::IndexedRandom;
use rand::{RngExt, SeedableRng};
use rand_distr::{Beta, StandardNormal};

use crate::genre;

struct Country {
    code: &'static str,
    /// Share of creators based here.
    weight: u32,
    first_names: &'static [&'static str],
    last_names: &'static [&'static str],
    language: &'static str,
    /// Other languages, each with the share of accounts that also post in it.
    extra_languages: &'static [(&'static str, f64)],
}

/// Weighted towards Germany, where most of the marketplace's creators are.
static COUNTRIES: [Country; 4] = [
    Country {
        code: "DE",
        weight: 50,
        first_names: DE_DE::NAME_FIRST_NAME,
        last_names: DE_DE::NAME_LAST_NAME,
        language: "de",
        extra_languages: &[("en", 0.25), ("tr", 0.03)],
    },
    Country {
        code: "AT",
        weight: 10,
        first_names: DE_DE::NAME_FIRST_NAME,
        last_names: DE_DE::NAME_LAST_NAME,
        language: "de",
        extra_languages: &[("en", 0.2)],
    },
    Country {
        code: "GB",
        weight: 20,
        first_names: EN::NAME_FIRST_NAME,
        last_names: EN::NAME_LAST_NAME,
        language: "en",
        extra_languages: &[("fr", 0.02)],
    },
    Country {
        code: "US",
        weight: 20,
        first_names: EN::NAME_FIRST_NAME,
        last_names: EN::NAME_LAST_NAME,
        language: "en",
        extra_languages: &[("es", 0.15)],
    },
];

/// Share of accounts whose country is drawn independently of their creator's.
const OWN_COUNTRY: f64 = 0.05;

/// Every genre, weighted by how many accounts have it as their main genre.
static GENRES: [(i16, u32); 21] = [
    (genre::MUSIC, 9),
    (genre::GAMING, 9),
    (genre::LIFESTYLE, 14),
    (genre::TRAVEL, 5),
    (genre::SPORTS, 5),
    (genre::FOOD, 8),
    (genre::COMEDY, 11),
    (genre::BEAUTY, 10),
    (genre::TECH, 4),
    (genre::CARS, 2),
    (genre::BUSINESS, 2),
    (genre::FILM_CINEMA, 3),
    (genre::ART, 2),
    (genre::FITNESS, 8),
    (genre::STREAMING, 3),
    (genre::TIKTOK, 4),
    (genre::MEMES, 5),
    (genre::NIGHTLIFE, 2),
    (genre::BOOKS_WRITING, 1),
    (genre::TUTORIALS, 3),
    (genre::OTHER, 3),
];

/// Weights of 1, 2 and 3 neighbours of the main genre per account.
const NEIGHBOURS_PER_ACCOUNT: [u32; 3] = [40, 40, 20];
/// Weights of 1 to 4 accounts per creator, about 1.5 on average.
const ACCOUNTS_PER_CREATOR: [u32; 4] = [65, 25, 7, 3];
const TIKTOK_FIRST: f64 = 0.6;
/// Chance that each further account is on the previous one's platform, so most creators with two
/// accounts have one on each.
const SAME_PLATFORM_AGAIN: f64 = 0.2;

struct PlatformStats {
    median_views: f64,
    /// Typical `view_score` per follower.
    views_per_follower: f64,
    /// Typical engagement rate of an account with 1,000 views per post.
    engagement_at_1k_views: f64,
    median_posts_per_week: f64,
}

const TIKTOK: PlatformStats = PlatformStats {
    median_views: 2_000.0,
    // The For You feed shows videos far beyond the followers.
    views_per_follower: 0.25,
    engagement_at_1k_views: 0.09,
    median_posts_per_week: 4.0,
};

const INSTAGRAM: PlatformStats = PlatformStats {
    median_views: 1_200.0,
    // Reels reach a smaller share of the followers.
    views_per_follower: 0.08,
    engagement_at_1k_views: 0.07,
    median_posts_per_week: 2.5,
};

/// Standard deviation of ln(`view_score`). With the medians above, about 28% of accounts are
/// starters (under 500), 32% nanos (to 3k), 30% micros (to 30k), 9% macros (to 300k) and 1%
/// megas.
const VIEWS_SIGMA: f64 = 2.3;
const VIEWS_RANGE: RangeInclusive<f64> = 50.0..=20_000_000.0;
/// Correlation between the sizes of one creator's accounts.
const SIZE_CORRELATION: f64 = 0.7;
/// Log-normal spread of followers, engagement and posting around their typical values.
const FOLLOWERS_SIGMA: f64 = 0.6;
const ENGAGEMENT_SIGMA: f64 = 0.35;
const POSTING_SIGMA: f64 = 0.5;
/// Bigger accounts engage less: ten times the views, 0.76 times the engagement rate.
const ENGAGEMENT_SIZE_EXPONENT: f64 = -0.12;
/// Engagement rates and posting frequencies are clamped to these.
const ENGAGEMENT_RATE_RANGE: RangeInclusive<f64> = 0.001..=0.5;
const POSTS_PER_WEEK_RANGE: RangeInclusive<f64> = 0.5..=14.0;
/// Content quality is normal around 60 with a standard deviation of 14, plus 4 points per
/// standard deviation of size, as bigger accounts tend to have better production.
const QUALITY_MEDIAN: f64 = 60.0;
const QUALITY_SIGMA: f64 = 14.0;
const QUALITY_PER_SIZE: f64 = 4.0;
/// Share of accounts that are brand safe; campaigns never match the others.
const BRAND_SAFE: f64 = 0.95;
/// Reliability is Beta(8, 2) scaled to 100: mean 80, four in five creators at 70 or above.
const RELIABILITY_BETA: (f64, f64) = (8.0, 2.0);

const DAY: i64 = 24 * 60 * 60;
const MAX_CREATOR_AGE: i64 = 3 * 365 * DAY;
/// Account stats are refreshed at least this often.
const MAX_STATS_AGE: i64 = 10 * DAY;
/// The stricter platform's limit; the other allows 30.
const MAX_HANDLE_LEN: usize = 24;

#[derive(Debug)]
pub struct SeedCreator {
    pub creator: Creator,
    pub accounts: Vec<SeedAccount>,
}

#[derive(Debug)]
pub struct SeedAccount {
    pub account: PlatformAccount,
    pub genre_ids: Vec<i16>,
    pub language_codes: Vec<&'static str>,
}

/// An endless iterator of creators with ids 1, 2, 3, ..., and accounts numbered the same way.
/// The same seed and `now` always give the same creators.
pub struct Generator {
    // A named algorithm, unlike `StdRng`, whose output may change between rand versions.
    rng: ChaCha8Rng,
    now: DateTime<Utc>,
    next_creator_id: i64,
    next_account_id: i64,
    countries: WeightedIndex<u32>,
    accounts_per_creator: WeightedIndex<u32>,
    genres: WeightedIndex<u32>,
    neighbours_per_account: WeightedIndex<u32>,
    reliability: Beta<f64>,
}

impl Generator {
    pub fn new(seed: u64, now: DateTime<Utc>) -> Self {
        Self {
            rng: ChaCha8Rng::seed_from_u64(seed),
            now,
            next_creator_id: 1,
            next_account_id: 1,
            countries: weighted(COUNTRIES.iter().map(|country| country.weight)),
            accounts_per_creator: weighted(ACCOUNTS_PER_CREATOR),
            genres: weighted(GENRES.iter().map(|&(_, weight)| weight)),
            neighbours_per_account: weighted(NEIGHBOURS_PER_ACCOUNT),
            reliability: Beta::new(RELIABILITY_BETA.0, RELIABILITY_BETA.1)
                .expect("parameters are positive"),
        }
    }

    fn account(
        &mut self,
        creator_id: i64,
        platform: Platform,
        country: &'static Country,
        handle_base: &str,
        creator_size: f64,
        age: i64,
    ) -> SeedAccount {
        let id = self.next_account_id;
        self.next_account_id += 1;
        let stats = match platform {
            Platform::TikTok => &TIKTOK,
            Platform::Instagram => &INSTAGRAM,
        };

        let (size, view_score) = self.view_score(stats, creator_size);
        let views = f64::from(view_score);
        let followers =
            round(views / (stats.views_per_follower * self.log_normal(FOLLOWERS_SIGMA)));
        let engagement_rate = stats.engagement_at_1k_views
            * (views / 1_000.0).powf(ENGAGEMENT_SIZE_EXPONENT)
            * self.log_normal(ENGAGEMENT_SIGMA);
        let posts_per_week = stats.median_posts_per_week * self.log_normal(POSTING_SIGMA);
        let quality =
            QUALITY_MEDIAN + QUALITY_PER_SIZE * size + QUALITY_SIGMA * self.standard_normal();
        let language_codes = self.language_codes(country);
        let genre_ids = self.genre_ids();
        let brand_safe = self.rng.random_bool(BRAND_SAFE);
        let stats_age = self.rng.random_range(0..MAX_STATS_AGE);

        SeedAccount {
            account: PlatformAccount {
                id,
                creator_id,
                platform,
                handle: handle(handle_base, id),
                country_code: country.code.to_owned(),
                followers,
                view_score,
                engagement_rate: clamp_and_round(engagement_rate, ENGAGEMENT_RATE_RANGE, 4),
                posts_per_week: clamp_and_round(posts_per_week, POSTS_PER_WEEK_RANGE, 1),
                quality_score: score(quality),
                brand_safe,
                stats_updated_at: self.now - TimeDelta::seconds(stats_age),
                created_at: self.now - TimeDelta::seconds(age),
            },
            genre_ids,
            language_codes,
        }
    }

    /// Log-normal views around the platform's median, redrawn until within [`VIEWS_RANGE`], and
    /// the account's size in standard deviations, partly shared with its creator's other accounts.
    fn view_score(&mut self, stats: &PlatformStats, creator_size: f64) -> (f64, i32) {
        loop {
            let size = SIZE_CORRELATION * creator_size
                + (1.0 - SIZE_CORRELATION.powi(2)).sqrt() * self.standard_normal();
            let views = stats.median_views * (VIEWS_SIGMA * size).exp();
            if VIEWS_RANGE.contains(&views) {
                return (size, round(views));
            }
        }
    }

    fn language_codes(&mut self, country: &Country) -> Vec<&'static str> {
        let mut codes = vec![country.language];
        for &(code, share) in country.extra_languages {
            if self.rng.random_bool(share) {
                codes.push(code);
            }
        }
        codes
    }

    /// A main genre, then one to three of its neighbours; the common genres are more likely in
    /// both. Other has no neighbours, so an account whose main genre is Other has only that one.
    fn genre_ids(&mut self) -> Vec<i16> {
        let (main, _) = GENRES[self.genres.sample(&mut self.rng)];
        let count = self.neighbours_per_account.sample(&mut self.rng) + 1;
        let mut ids = vec![main];
        ids.extend(
            neighbours(main)
                .sample_weighted(&mut self.rng, count, |&id| genre_weight(id))
                .expect("weights are positive")
                .copied(),
        );
        ids
    }

    fn standard_normal(&mut self) -> f64 {
        self.rng.sample(StandardNormal)
    }

    /// A factor with median 1.
    fn log_normal(&mut self, sigma: f64) -> f64 {
        (sigma * self.standard_normal()).exp()
    }
}

impl Iterator for Generator {
    type Item = SeedCreator;

    fn next(&mut self) -> Option<SeedCreator> {
        let home = &COUNTRIES[self.countries.sample(&mut self.rng)];
        let first = home.first_names.choose(&mut self.rng).expect("not empty");
        let last = home.last_names.choose(&mut self.rng).expect("not empty");
        // Joined early enough for every account to predate its stats.
        let age = self.rng.random_range(MAX_STATS_AGE + DAY..=MAX_CREATOR_AGE);
        let creator = Creator {
            id: self.next_creator_id,
            name: format!("{first} {last}"),
            reliability_score: score(100.0 * self.reliability.sample(&mut self.rng)),
            created_at: self.now - TimeDelta::seconds(age),
        };
        self.next_creator_id += 1;

        let separator = ["", ".", "_"].choose(&mut self.rng).expect("not empty");
        let handle_base = handle_base(first, last, separator);
        let size = self.standard_normal();
        let mut platform = if self.rng.random_bool(TIKTOK_FIRST) {
            Platform::TikTok
        } else {
            Platform::Instagram
        };
        let account_count = self.accounts_per_creator.sample(&mut self.rng) + 1;
        let mut accounts = Vec::with_capacity(account_count);
        for i in 0..account_count {
            if i > 0 && !self.rng.random_bool(SAME_PLATFORM_AGAIN) {
                platform = match platform {
                    Platform::TikTok => Platform::Instagram,
                    Platform::Instagram => Platform::TikTok,
                };
            }
            let country = if self.rng.random_bool(OWN_COUNTRY) {
                &COUNTRIES[self.countries.sample(&mut self.rng)]
            } else {
                home
            };
            // The first account is connected at sign-up, later ones before the oldest stats.
            let account_age = if i == 0 {
                age
            } else {
                self.rng.random_range(MAX_STATS_AGE..=age)
            };
            accounts.push(self.account(
                creator.id,
                platform,
                country,
                &handle_base,
                size,
                account_age,
            ));
        }

        Some(SeedCreator { creator, accounts })
    }
}

fn weighted(weights: impl IntoIterator<Item = u32>) -> WeightedIndex<u32> {
    WeightedIndex::new(weights).expect("weights are positive")
}

fn neighbours(main: i16) -> &'static [i16] {
    genre::NEIGHBOURS
        .iter()
        .find(|&&(id, _)| id == main)
        .map(|&(_, neighbours)| neighbours)
        .expect("every genre has a row in NEIGHBOURS")
}

fn genre_weight(genre_id: i16) -> u32 {
    GENRES
        .iter()
        .find(|&&(id, _)| id == genre_id)
        .map(|&(_, weight)| weight)
        .expect("every genre has a row in GENRES")
}

/// Values beyond the `i32` range become its bounds: `as` saturates.
#[expect(clippy::cast_possible_truncation, reason = "saturating is intended")]
fn round(x: f64) -> i32 {
    x.round() as i32
}

#[expect(clippy::cast_possible_truncation, reason = "clamped to 0..=100 first")]
fn score(x: f64) -> i16 {
    x.round().clamp(0.0, 100.0) as i16
}

fn clamp_and_round(x: f64, range: RangeInclusive<f64>, decimals: i32) -> f64 {
    let scale = 10_f64.powi(decimals);
    (x.clamp(*range.start(), *range.end()) * scale).round() / scale
}

fn handle_base(first: &str, last: &str, separator: &str) -> String {
    let letters = |name: &str| {
        deunicode(name)
            .chars()
            .filter(char::is_ascii_alphabetic)
            .map(|c| c.to_ascii_lowercase())
            .collect::<String>()
    };
    format!("{}{separator}{}", letters(first), letters(last))
}

/// Ends in the account id, which makes it unique across both platforms: the base has no digits,
/// so two handles can only be equal if their ids are.
fn handle(base: &str, account_id: i64) -> String {
    let id = account_id.to_string();
    // The base is ASCII, so any byte index is a char boundary.
    let base = &base[..base.len().min(MAX_HANDLE_LEN.saturating_sub(id.len()))];
    format!("{base}{id}")
}

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashSet};

    use marketplace::SizeGroup::{self, Macro, Mega, Micro, Nano, Starter};

    use super::*;
    use crate::tests::now;

    #[test]
    fn handles_are_unique_across_platforms() {
        let mut handles = HashSet::new();
        for creator in Generator::new(1, now()).take(10_000) {
            for SeedAccount { account, .. } in creator.accounts {
                assert!(handles.insert(account.handle.clone()), "{}", account.handle);
            }
        }
    }

    #[test]
    fn each_account_has_a_main_genre_and_one_to_three_of_its_neighbours() {
        for creator in Generator::new(1, now()).take(10_000) {
            for SeedAccount { genre_ids, .. } in creator.accounts {
                let (&main, others) = genre_ids.split_first().expect("a main genre");
                if main == genre::OTHER {
                    assert!(others.is_empty(), "{genre_ids:?}");
                } else {
                    assert!((1..=3).contains(&others.len()), "{genre_ids:?}");
                    let related = others.iter().all(|id| neighbours(main).contains(id));
                    assert!(related, "{genre_ids:?}");
                }
            }
        }
    }

    #[test]
    fn account_sizes_have_the_shares_views_sigma_promises() {
        let mut counts = BTreeMap::<SizeGroup, usize>::new();
        for creator in Generator::new(1, now()).take(10_000) {
            for SeedAccount { account, .. } in creator.accounts {
                *counts.entry(SizeGroup::of(account.view_score)).or_default() += 1;
            }
        }
        let total: usize = counts.values().sum();
        // The shares in VIEWS_SIGMA's doc, in percent, give or take a point.
        for (group, percent) in [
            (Starter, 28),
            (Nano, 32),
            (Micro, 30),
            (Macro, 9),
            (Mega, 1),
        ] {
            let per_mille = counts[&group] * 1_000 / total;
            assert!(
                per_mille.abs_diff(percent * 10) <= 10,
                "{group:?}: {per_mille} in 1,000 accounts"
            );
        }
    }
}
