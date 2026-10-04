//! Fake advertisers, each named after a brand and the category it sells.

use std::ops::RangeInclusive;

use chrono::{DateTime, TimeDelta, Utc};
use db::models::Advertiser;
use rand::rngs::ChaCha8Rng;
use rand::seq::SliceRandom;
use rand::{RngExt, SeedableRng};

use crate::data::{BRANDS, CATEGORIES};

/// Names are distinct: one per brand and category pair.
pub const MAX_ADVERTISERS: usize = BRANDS.len() * CATEGORIES.len();

const DAY: i64 = 24 * 60 * 60;
/// Advertisers joined from a day to three years before the seeding, in seconds.
const JOINED_SECONDS_AGO: RangeInclusive<i64> = DAY..=3 * 365 * DAY;

/// An advertiser and what it sells, which its campaigns promote.
#[derive(Debug)]
pub struct SeedAdvertiser {
    pub advertiser: Advertiser,
    /// Its index in [`CATEGORIES`], whose name ends the advertiser's.
    pub category: usize,
}

/// `count` advertisers with ids 1 to `count` and distinct names, at most [`MAX_ADVERTISERS`].
pub fn advertisers(seed: u64, count: usize, now: DateTime<Utc>) -> Vec<SeedAdvertiser> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // Each generator has its own stream of the seed (creators 0, advertisers 1, campaigns 2), so
    // they don't draw the same numbers.
    rng.set_stream(1);
    let mut names: Vec<_> = BRANDS
        .iter()
        .flat_map(|&brand| (0..CATEGORIES.len()).map(move |category| (brand, category)))
        .collect();
    names.shuffle(&mut rng);
    names
        .into_iter()
        .take(count)
        .zip(1..)
        .map(|((brand, category), id)| SeedAdvertiser {
            advertiser: Advertiser {
                id,
                name: format!("{brand} {}", CATEGORIES[category].name),
                created_at: now - TimeDelta::seconds(rng.random_range(JOINED_SECONDS_AGO)),
            },
            category,
        })
        .collect()
}
