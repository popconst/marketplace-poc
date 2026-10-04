//! Fake campaigns. What a campaign promotes follows its advertiser's category; who it targets
//! and what it pays come from [`CAMPAIGN_TERMS`].

use chrono::{DateTime, TimeDelta, Utc};
use db::models::Platform;
use marketplace::SizeGroup::{self, Macro, Micro, Nano};
use rand::rngs::ChaCha8Rng;
use rand::seq::IndexedRandom;
use rand::{RngExt, SeedableRng};

use crate::advertisers::SeedAdvertiser;
use crate::data::{CAMPAIGN_TERMS, CATEGORIES, Pitch, Sliders, Terms};

/// Campaigns were created in the last month, but not before their advertiser joined.
const MAX_CAMPAIGN_AGE: TimeDelta = TimeDelta::days(30);
const BIDDING_PERIOD: TimeDelta = TimeDelta::days(3);
/// A new draft's sizes and sliders: the database's defaults.
const DEFAULT_SIZE_GROUPS: &[SizeGroup] = &[Nano, Micro, Macro];
const DEFAULT_SLIDERS: Sliders = Sliders {
    engagement: 50,
    quality: 50,
    reliability: 50,
};

/// How much of the form a draft has filled in. The form asks for the title, the briefing, the
/// platform and money, then the targeting, and a draft may stop after any step.
#[derive(Clone, Copy)]
struct FilledIn {
    briefing: bool,
    /// The platform, the budget and the target CPM.
    money: bool,
    /// The countries, languages, genres, sizes and sliders.
    targeting: bool,
}

/// How far a draft gets, from only a title to everything but the deadline, each as likely.
const DRAFT_STAGES: [FilledIn; 4] = [
    FilledIn {
        briefing: false,
        money: false,
        targeting: false,
    },
    FilledIn {
        briefing: true,
        money: false,
        targeting: false,
    },
    FilledIn {
        briefing: true,
        money: true,
        targeting: false,
    },
    FilledIn {
        briefing: true,
        money: true,
        targeting: true,
    },
];

/// A campaign with its targeting. Money is in euro cents.
#[derive(Debug)]
pub struct SeedCampaign {
    pub id: i64,
    pub advertiser_id: i64,
    pub title: &'static str,
    pub briefing: Option<&'static str>,
    pub platform: Option<Platform>,
    pub budget_cents: Option<i64>,
    pub target_cpm_cents: Option<i64>,
    pub size_groups: &'static [SizeGroup],
    pub sliders: Sliders,
    /// Empty for any.
    pub country_codes: &'static [&'static str],
    /// Empty for any.
    pub language_codes: &'static [&'static str],
    /// Empty for any.
    pub genre_ids: &'static [i16],
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    /// `None` for a draft.
    pub published: Option<Published>,
}

#[derive(Debug, Clone, Copy)]
pub struct Published {
    pub at: DateTime<Utc>,
    pub bidding_deadline: DateTime<Utc>,
}

/// Campaigns with ids 1, 2, 3, ...: for each advertiser, one published at `now`, then one or two
/// drafts. An advertiser's campaigns all have different pitches, and so do the open campaigns of
/// up to three advertisers in one category.
pub fn campaigns(
    seed: u64,
    advertisers: &[SeedAdvertiser],
    now: DateTime<Utc>,
) -> Vec<SeedCampaign> {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    // See `advertisers` for why each generator has its own stream.
    rng.set_stream(2);
    let mut campaigns = Vec::new();
    let mut next_id = 1;
    // How many advertisers of each category came before.
    let mut earlier_in_category = [0; CATEGORIES.len()];
    for (seeded, open_terms) in advertisers.iter().zip(CAMPAIGN_TERMS.iter().cycle()) {
        let advertiser = &seeded.advertiser;
        let pitches = &CATEGORIES[seeded.category].pitches;
        // The nth advertiser of a category starts at its nth pitch.
        let first_pitch = earlier_in_category[seeded.category];
        earlier_in_category[seeded.category] += 1;
        let earliest = advertiser.created_at.max(now - MAX_CAMPAIGN_AGE);

        let draft_count = rng.random_range(1..=2);
        for n in 0..=draft_count {
            let pitch = &pitches[(first_pitch + n) % pitches.len()];
            let created_at = between(&mut rng, earliest, now);
            campaigns.push(if n == 0 {
                open_campaign(next_id, advertiser.id, pitch, open_terms, created_at, now)
            } else {
                draft_campaign(&mut rng, next_id, advertiser.id, pitch, created_at, now)
            });
            next_id += 1;
        }
    }
    campaigns
}

fn open_campaign(
    id: i64,
    advertiser_id: i64,
    pitch: &'static Pitch,
    terms: &'static Terms,
    created_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> SeedCampaign {
    SeedCampaign {
        id,
        advertiser_id,
        title: pitch.title,
        briefing: Some(pitch.briefing),
        platform: Some(terms.platform),
        budget_cents: Some(terms.budget_cents),
        target_cpm_cents: Some(terms.target_cpm_cents),
        size_groups: terms.size_groups,
        sliders: terms.sliders,
        country_codes: terms.countries,
        language_codes: terms.languages,
        genre_ids: pitch.genre_ids,
        created_at,
        updated_at: now,
        published: Some(Published {
            at: now,
            bidding_deadline: now + BIDDING_PERIOD,
        }),
    }
}

fn draft_campaign(
    rng: &mut ChaCha8Rng,
    id: i64,
    advertiser_id: i64,
    pitch: &'static Pitch,
    created_at: DateTime<Utc>,
    now: DateTime<Utc>,
) -> SeedCampaign {
    let terms = CAMPAIGN_TERMS.choose(rng).expect("not empty");
    let filled = *DRAFT_STAGES.choose(rng).expect("not empty");
    SeedCampaign {
        id,
        advertiser_id,
        title: pitch.title,
        briefing: filled.briefing.then_some(pitch.briefing),
        platform: filled.money.then_some(terms.platform),
        budget_cents: filled.money.then_some(terms.budget_cents),
        target_cpm_cents: filled.money.then_some(terms.target_cpm_cents),
        size_groups: if filled.targeting {
            terms.size_groups
        } else {
            DEFAULT_SIZE_GROUPS
        },
        sliders: if filled.targeting {
            terms.sliders
        } else {
            DEFAULT_SLIDERS
        },
        country_codes: if filled.targeting {
            terms.countries
        } else {
            &[]
        },
        language_codes: if filled.targeting {
            terms.languages
        } else {
            &[]
        },
        genre_ids: if filled.targeting {
            pitch.genre_ids
        } else {
            &[]
        },
        created_at,
        // Drafts are saved as the advertiser types.
        updated_at: between(rng, created_at, now),
        published: None,
    }
}

/// A time from `from` to `to`, both included, to the second.
fn between(rng: &mut ChaCha8Rng, from: DateTime<Utc>, to: DateTime<Utc>) -> DateTime<Utc> {
    from + TimeDelta::seconds(rng.random_range(0..=(to - from).num_seconds()))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;
    use crate::advertisers::{MAX_ADVERTISERS, advertisers};
    use crate::tests::now;

    #[test]
    fn each_advertiser_has_one_open_campaign_and_one_or_two_drafts() {
        let advertisers = advertisers(1, MAX_ADVERTISERS, now());
        let campaigns = campaigns(1, &advertisers, now());
        for seeded in &advertisers {
            let own: Vec<_> = campaigns
                .iter()
                .filter(|campaign| campaign.advertiser_id == seeded.advertiser.id)
                .collect();
            let open = own.iter().filter(|c| c.published.is_some()).count();
            assert_eq!(open, 1);
            assert!(
                (1..=2).contains(&(own.len() - open)),
                "{} drafts",
                own.len() - open
            );
            let titles: HashSet<_> = own.iter().map(|campaign| campaign.title).collect();
            assert_eq!(titles.len(), own.len());
        }
    }
}
