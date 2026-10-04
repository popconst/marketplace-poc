//! The business settings, read from environment variables with today's values as defaults. The
//! API and the seeder refuse to start if they do not fit together.
//!
//! The deal rules ([`Rules`]) are frozen on a campaign when it is published, so a later change
//! never alters a live or closed campaign. The lowest offer and the campaign limits only shape
//! drafts, so they are never frozen; a campaign keeps just the offer it was published with.

use clap::Args;
use marketplace::{DEFAULT_MIN_OFFER_BPS, Rules, SizeGroup, Terms};

/// Appended to a settings error to say where the setting comes from.
const VARIABLE_NAMES: &str =
    "each setting is read from the environment variable of its name in upper case";

/// The business settings, checked to fit together by [`SettingsArgs::into_settings`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub rules: Rules,
    /// The lowest offer, as a share of the usual rate.
    pub min_offer_bps: i64,
    pub limits: CampaignLimits,
}

/// What a draft may save or publish.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CampaignLimits {
    /// The earliest bidding deadline, in hours after publishing.
    pub min_bidding_hours: i64,
    /// The latest bidding deadline, in hours after publishing.
    pub max_bidding_hours: i64,
    /// The posting window a new draft starts with, in days.
    pub posting_window_default_days: i16,
    /// The longest posting window an advertiser may pick, in days.
    pub posting_window_max_days: i16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            rules: Rules::default(),
            min_offer_bps: DEFAULT_MIN_OFFER_BPS,
            limits: CampaignLimits::default(),
        }
    }
}

impl Default for CampaignLimits {
    fn default() -> Self {
        Self {
            min_bidding_hours: 24,
            max_bidding_hours: 720,
            posting_window_default_days: 3,
            posting_window_max_days: 7,
        }
    }
}

impl Settings {
    /// Checks that the settings fit together.
    ///
    /// # Errors
    ///
    /// Returns the first problem found, naming the setting.
    pub fn validate(&self) -> Result<(), String> {
        self.rules.validate()?;
        self.rules.validate_min_offer(self.min_offer_bps)?;
        self.limits.validate()
    }

    /// The offer these figures would be published with now, or `None` if no size is picked.
    #[must_use]
    pub fn offer_bps(&self, target_cpm_cents: i64, size_groups: &[SizeGroup]) -> Option<i64> {
        self.rules
            .offer_bps(target_cpm_cents, size_groups, self.min_offer_bps)
    }

    /// The terms these figures would be published with now, or `None` if no size is picked.
    #[must_use]
    pub fn terms(
        &self,
        budget_cents: i64,
        target_cpm_cents: i64,
        size_groups: &[SizeGroup],
    ) -> Option<Terms> {
        Some(Terms {
            budget_cents,
            size_groups: size_groups.to_vec(),
            offer_bps: self.offer_bps(target_cpm_cents, size_groups)?,
            rules: self.rules,
        })
    }
}

impl CampaignLimits {
    fn validate(&self) -> Result<(), String> {
        let (min, max) = (self.min_bidding_hours, self.max_bidding_hours);
        if min < 1 || min >= max {
            return Err(format!(
                "min_bidding_hours must be at least 1 and below max_bidding_hours, not {min} and \
                 {max}"
            ));
        }
        let (default, max) = (
            self.posting_window_default_days,
            self.posting_window_max_days,
        );
        if !(1..=max).contains(&default) {
            return Err(format!(
                "posting_window_default_days must be from 1 to posting_window_max_days, not \
                 {default} and {max}"
            ));
        }
        Ok(())
    }
}

/// The settings as environment variables or command-line options. [`Rules`] explains each rule.
#[derive(Debug, Clone, Args)]
pub struct SettingsArgs {
    /// WePush's commission on each winning bid, in basis points (1500 is 15%).
    #[arg(long, env = "COMMISSION_BPS", default_value_t = Rules::default().commission_bps)]
    commission_bps: i64,

    /// The fixed part of an account's usual rate, in cents. Also the lowest bid anyone may make.
    #[arg(
        long,
        env = "USUAL_RATE_BASE_CENTS",
        default_value_t = Rules::default().usual_rate_base_cents
    )]
    usual_rate_base_cents: i64,

    /// What each 1,000 expected views add to an account's usual rate, in cents.
    #[arg(
        long,
        env = "USUAL_RATE_PER_1000_VIEWS_CENTS",
        default_value_t = Rules::default().usual_rate_per_1000_views_cents
    )]
    usual_rate_per_1000_views_cents: i64,

    /// The lowest bid, as a share of the account's usual rate, in basis points.
    #[arg(
        long,
        env = "FAIR_PAY_FLOOR_BPS",
        default_value_t = Rules::default().fair_pay_floor_bps
    )]
    fair_pay_floor_bps: i64,

    /// The most one winning bid may take of the budget, in basis points.
    #[arg(
        long,
        env = "MAX_BID_SHARE_OF_BUDGET_BPS",
        default_value_t = Rules::default().max_bid_share_of_budget_bps
    )]
    max_bid_share_of_budget_bps: i64,

    /// The share of expected views a video must reach to be paid the usual rate, in basis points.
    #[arg(
        long,
        env = "VIEWS_TO_GET_PAID_BPS",
        default_value_t = Rules::default().views_to_get_paid_bps
    )]
    views_to_get_paid_bps: i64,

    /// The most views a bid may promise, as a share of the expected views, in basis points.
    #[arg(
        long,
        env = "LIKELY_VIEWS_LIMIT_BPS",
        default_value_t = Rules::default().likely_views_limit_bps
    )]
    likely_views_limit_bps: i64,

    /// The share of each size's budget set aside for weaker matches, in basis points.
    #[arg(long, env = "DISCOVERY_BPS", default_value_t = Rules::default().discovery_bps)]
    discovery_bps: i64,

    /// The lowest offer, as a share of the usual rate, in basis points.
    #[arg(long, env = "MIN_OFFER_BPS", default_value_t = DEFAULT_MIN_OFFER_BPS)]
    min_offer_bps: i64,

    /// The earliest bidding deadline, in hours after publishing.
    #[arg(
        long,
        env = "MIN_BIDDING_HOURS",
        default_value_t = CampaignLimits::default().min_bidding_hours
    )]
    min_bidding_hours: i64,

    /// The latest bidding deadline, in hours after publishing.
    #[arg(
        long,
        env = "MAX_BIDDING_HOURS",
        default_value_t = CampaignLimits::default().max_bidding_hours
    )]
    max_bidding_hours: i64,

    /// The posting window a new draft starts with, in days.
    #[arg(
        long,
        env = "POSTING_WINDOW_DEFAULT_DAYS",
        default_value_t = CampaignLimits::default().posting_window_default_days
    )]
    posting_window_default_days: i16,

    /// The longest posting window an advertiser may pick, in days.
    #[arg(
        long,
        env = "POSTING_WINDOW_MAX_DAYS",
        default_value_t = CampaignLimits::default().posting_window_max_days
    )]
    posting_window_max_days: i16,
}

impl SettingsArgs {
    /// Checks that the settings fit together and returns them.
    ///
    /// # Errors
    ///
    /// Returns the first problem found, naming the setting and saying where it is read from.
    pub fn into_settings(self) -> Result<Settings, String> {
        let settings = Settings {
            rules: Rules {
                commission_bps: self.commission_bps,
                usual_rate_base_cents: self.usual_rate_base_cents,
                usual_rate_per_1000_views_cents: self.usual_rate_per_1000_views_cents,
                fair_pay_floor_bps: self.fair_pay_floor_bps,
                max_bid_share_of_budget_bps: self.max_bid_share_of_budget_bps,
                views_to_get_paid_bps: self.views_to_get_paid_bps,
                likely_views_limit_bps: self.likely_views_limit_bps,
                discovery_bps: self.discovery_bps,
            },
            min_offer_bps: self.min_offer_bps,
            limits: CampaignLimits {
                min_bidding_hours: self.min_bidding_hours,
                max_bidding_hours: self.max_bidding_hours,
                posting_window_default_days: self.posting_window_default_days,
                posting_window_max_days: self.posting_window_max_days,
            },
        };
        settings
            .validate()
            .map_err(|problem| format!("{problem} ({VARIABLE_NAMES})"))?;
        Ok(settings)
    }
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        settings: SettingsArgs,
    }

    fn parse(options: &[&str]) -> Result<Settings, String> {
        let cli = Cli::try_parse_from(["test"].iter().chain(options)).map_err(|e| e.to_string())?;
        cli.settings.into_settings()
    }

    #[test]
    fn settings_default_to_todays_values() {
        assert_eq!(parse(&[]), Ok(Settings::default()));
    }

    #[test]
    fn each_option_sets_its_own_setting() {
        let settings = parse(&[
            "--commission-bps=2000",
            "--usual-rate-base-cents=8000",
            "--usual-rate-per-1000-views-cents=1100",
            "--fair-pay-floor-bps=4000",
            "--max-bid-share-of-budget-bps=2000",
            "--views-to-get-paid-bps=3000",
            "--likely-views-limit-bps=6500",
            "--discovery-bps=1000",
            "--min-offer-bps=4500",
            "--min-bidding-hours=12",
            "--max-bidding-hours=240",
            "--posting-window-default-days=2",
            "--posting-window-max-days=5",
        ]);

        let rules = Rules {
            commission_bps: 2000,
            usual_rate_base_cents: 8000,
            usual_rate_per_1000_views_cents: 1100,
            fair_pay_floor_bps: 4000,
            max_bid_share_of_budget_bps: 2000,
            views_to_get_paid_bps: 3000,
            likely_views_limit_bps: 6500,
            discovery_bps: 1000,
        };
        let limits = CampaignLimits {
            min_bidding_hours: 12,
            max_bidding_hours: 240,
            posting_window_default_days: 2,
            posting_window_max_days: 5,
        };
        assert_eq!(
            settings,
            Ok(Settings {
                rules,
                min_offer_bps: 4500,
                limits,
            })
        );
    }

    #[test]
    fn settings_that_do_not_fit_together_stop_the_start() {
        for (option, setting) in [
            ("--views-to-get-paid-bps=7000", "likely_views_limit_bps"),
            ("--min-offer-bps=4000", "min_offer_bps"),
            ("--min-bidding-hours=0", "min_bidding_hours"),
            ("--min-bidding-hours=720", "min_bidding_hours"),
            (
                "--posting-window-default-days=0",
                "posting_window_default_days",
            ),
            (
                "--posting-window-default-days=8",
                "posting_window_default_days",
            ),
        ] {
            let problem = parse(&[option]).unwrap_err();
            assert!(problem.starts_with(setting), "{option}: {problem}");
            assert!(
                problem.ends_with(&format!("({VARIABLE_NAMES})")),
                "{problem}"
            );
        }
    }
}
