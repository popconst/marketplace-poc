//! Plain data for `advertisers` and `campaigns`: what advertisers sell, and campaign terms.

use db::models::Platform::{self, Instagram, TikTok};
use marketplace::SizeGroup::{self, Macro, Mega, Micro, Nano};

use crate::genre;

/// What an advertiser sells, with three campaign pitches. An advertiser's name is a brand
/// followed by this name.
pub struct Category {
    pub name: &'static str,
    pub pitches: [Pitch; 3],
}

/// What a campaign asks creators to post.
pub struct Pitch {
    pub title: &'static str,
    pub briefing: &'static str,
    /// Empty for any genre.
    pub genre_ids: &'static [i16],
}

pub static CATEGORIES: [Category; 16] = [
    Category {
        name: "Apparel",
        pitches: [
            Pitch {
                title: "Autumn layers, your way",
                briefing: "Style three outfits around our new heavyweight overshirt and show how \
                           you wear it from morning coffee to evening plans. Real wardrobes, no \
                           studio needed.",
                genre_ids: &[genre::LIFESTYLE, genre::ART, genre::TIKTOK],
            },
            Pitch {
                title: "Ready for the night",
                briefing: "Get ready with us for a night out in pieces from our evening \
                           collection. Show the outfit, the music and the moment you head out.",
                genre_ids: &[genre::NIGHTLIFE, genre::MUSIC, genre::LIFESTYLE],
            },
            Pitch {
                title: "One jacket, five looks",
                briefing: "Style our recycled rain jacket five ways in under a minute. Quick cuts, \
                           and a word on the recycled fabric.",
                genre_ids: &[genre::LIFESTYLE, genre::TUTORIALS],
            },
        ],
    },
    Category {
        name: "Audio",
        pitches: [
            Pitch {
                title: "Hear the difference",
                briefing: "Play your favourite album, film or game on our new wireless \
                           headphones and tell your audience what you noticed for the first time. \
                           Mention the 40-hour battery.",
                genre_ids: &[genre::MUSIC, genre::TECH, genre::FILM_CINEMA],
            },
            Pitch {
                title: "Noise off, focus on",
                briefing: "Show a study or deep-work session with our noise-cancelling earbuds: \
                           the noisy café before, the quiet after. Keep it unscripted.",
                genre_ids: &[genre::TECH, genre::BOOKS_WRITING, genre::BUSINESS],
            },
            Pitch {
                title: "Soundtrack your workout",
                briefing: "Film a workout with our sweat-proof sport earbuds and share the \
                           playlist that gets you through it. Show that they stay in.",
                genre_ids: &[genre::FITNESS, genre::MUSIC, genre::SPORTS],
            },
        ],
    },
    Category {
        name: "Bikes",
        pitches: [
            Pitch {
                title: "Ride to work week",
                briefing: "Swap one commute by car for a ride on our city e-bike and film the \
                           whole trip, door to desk. Tell us how it compared.",
                genre_ids: &[genre::SPORTS, genre::CARS, genre::BUSINESS],
            },
            Pitch {
                title: "Weekend trail ride",
                briefing: "Take our gravel bike out of the city for a weekend loop and show the \
                           route, the stops and the climbs. Share the route so others can ride it.",
                genre_ids: &[genre::SPORTS, genre::TRAVEL, genre::FITNESS],
            },
            Pitch {
                title: "Fix a flat in two minutes",
                briefing: "Teach your audience to fix a flat tyre with our pocket repair kit, \
                           start to finish, in one take if you can.",
                genre_ids: &[genre::TUTORIALS, genre::SPORTS],
            },
        ],
    },
    Category {
        name: "Coffee",
        pitches: [
            Pitch {
                title: "Cold brew mornings",
                briefing: "Show the first hour of your day with our oat milk cold brew: the \
                           commute, the book, the inbox. Mention that it has no added sugar.",
                genre_ids: &[genre::FOOD, genre::LIFESTYLE, genre::BOOKS_WRITING],
            },
            Pitch {
                title: "Espresso at your desk",
                briefing: "Unbox our compact espresso machine and pull your first shot at home or \
                           in the office. Show how long it takes and be honest about the taste.",
                genre_ids: &[genre::BUSINESS, genre::TECH, genre::FOOD],
            },
            Pitch {
                title: "Latte art at home",
                briefing: "Teach one simple latte art pattern with our milk frother and house \
                           blend. Failed attempts are welcome; they make it real.",
                genre_ids: &[genre::ART, genre::FOOD, genre::TUTORIALS],
            },
        ],
    },
    Category {
        name: "Cosmetics",
        pitches: [
            Pitch {
                title: "Five-minute festival look",
                briefing: "Create a bold festival look with our glitter liner and long-wear \
                           palette in five minutes. Show it holding up at the end of the night.",
                genre_ids: &[genre::BEAUTY, genre::NIGHTLIFE, genre::MUSIC],
            },
            Pitch {
                title: "The bold lip challenge",
                briefing: "Try our new matte lipstick in the boldest shade you dare and let your \
                           audience react. Keep it fun and show the swatch up close.",
                genre_ids: &[genre::BEAUTY, genre::TIKTOK, genre::COMEDY],
            },
            Pitch {
                title: "Makeup for beginners",
                briefing: "Walk a complete beginner through an everyday look with just five of our \
                           products. Slow down on the steps people usually skip.",
                genre_ids: &[genre::BEAUTY, genre::TUTORIALS],
            },
        ],
    },
    Category {
        name: "Drinks",
        pitches: [
            Pitch {
                title: "Zero sugar, all summer",
                briefing: "Take a cold can of our zero-sugar lemonade wherever your summer goes, \
                           and make it part of the moment rather than the subject.",
                genre_ids: &[],
            },
            Pitch {
                title: "Game-day refresh",
                briefing: "Watch or play the big game with our sparkling water close at hand. Show \
                           the reactions, the snacks and the drinks.",
                genre_ids: &[genre::SPORTS, genre::GAMING, genre::COMEDY],
            },
            Pitch {
                title: "Mocktail hour",
                briefing: "Mix one alcohol-free cocktail with our ginger tonic and share the \
                           recipe. Bonus points for a party setting.",
                genre_ids: &[genre::FOOD, genre::NIGHTLIFE, genre::TUTORIALS],
            },
        ],
    },
    Category {
        name: "Fitness",
        pitches: [
            Pitch {
                title: "30-day home workout challenge",
                briefing: "Start our 30-day challenge with the resistance band set and post day \
                           one: the setup, the workout, your honest verdict. No gym needed.",
                genre_ids: &[genre::FITNESS, genre::TIKTOK, genre::SPORTS],
            },
            Pitch {
                title: "What's in my gym bag",
                briefing: "Show what you pack for the gym, our shaker and recovery shake \
                           included, and why each thing earns its place.",
                genre_ids: &[genre::FITNESS, genre::LIFESTYLE],
            },
            Pitch {
                title: "Stretch after work",
                briefing: "Lead a ten-minute stretch on our yoga mat for people who sit all day. \
                           Keep the moves simple enough to follow at home.",
                genre_ids: &[genre::FITNESS, genre::TUTORIALS, genre::BUSINESS],
            },
        ],
    },
    Category {
        name: "Games",
        pitches: [
            Pitch {
                title: "Launch week: play our co-op game",
                briefing: "Stream or record your first hour in our new co-op adventure, ideally \
                           with a friend. Show the moments that made you laugh or shout.",
                genre_ids: &[genre::GAMING, genre::STREAMING, genre::MEMES],
            },
            Pitch {
                title: "Your funniest fail",
                briefing: "Play our party game with friends and share your funniest fail. Short \
                           and chaotic is perfect.",
                genre_ids: &[genre::GAMING, genre::COMEDY, genre::TIKTOK],
            },
            Pitch {
                title: "Speedrun the first level",
                briefing: "Race through the first level of our puzzle platformer and show your \
                           fastest route, with tips for beating your time.",
                genre_ids: &[genre::GAMING, genre::STREAMING, genre::TUTORIALS],
            },
        ],
    },
    Category {
        name: "Kitchen",
        pitches: [
            Pitch {
                title: "One-pan dinners",
                briefing: "Cook a weeknight dinner in our non-stick pan in under 30 minutes, start \
                           to finish. Show the clean-up too.",
                genre_ids: &[genre::FOOD, genre::TUTORIALS],
            },
            Pitch {
                title: "Meal prep Sunday",
                briefing: "Prep the week's lunches with our glass containers and show how you \
                           plan, cook and portion them.",
                genre_ids: &[genre::FOOD, genre::FITNESS, genre::LIFESTYLE],
            },
            Pitch {
                title: "Gadget or gimmick?",
                briefing: "Test our spiralizer, garlic press and herb scissors on camera and give \
                           your honest verdict on each.",
                genre_ids: &[genre::FOOD, genre::TECH, genre::COMEDY],
            },
        ],
    },
    Category {
        name: "Labs",
        pitches: [
            Pitch {
                title: "Build your dream desk setup",
                briefing: "Show how our smart desk lamp and wireless charger fit into your setup, \
                           and walk through the features you actually use.",
                genre_ids: &[genre::TECH, genre::BUSINESS, genre::TUTORIALS],
            },
            Pitch {
                title: "Study with us",
                briefing: "Run a study or writing session with our focus app's timer on screen, \
                           and talk about what helps you concentrate.",
                genre_ids: &[genre::TECH, genre::BOOKS_WRITING, genre::LIFESTYLE],
            },
            Pitch {
                title: "Smart home in a weekend",
                briefing: "Set up our smart plug starter kit from unboxing to first automation, \
                           and show one routine that saves you time.",
                genre_ids: &[genre::TECH, genre::TUTORIALS, genre::OTHER],
            },
        ],
    },
    Category {
        name: "Outdoor",
        pitches: [
            Pitch {
                title: "Road trip ready",
                briefing: "Pack the car for a weekend road trip with our camping gear, and show \
                           the drive, the pitch and the first evening.",
                genre_ids: &[genre::TRAVEL, genre::CARS, genre::OTHER],
            },
            Pitch {
                title: "First night under the stars",
                briefing: "Spend a night outdoors in our ultralight tent, even if it's in your \
                           garden, and share what surprised you.",
                genre_ids: &[genre::TRAVEL, genre::SPORTS, genre::TUTORIALS],
            },
            Pitch {
                title: "Rain or shine hike",
                briefing: "Take our waterproof jacket on a hike in the worst weather you can find, \
                           and show whether you stay dry.",
                genre_ids: &[genre::TRAVEL, genre::FITNESS, genre::LIFESTYLE],
            },
        ],
    },
    Category {
        name: "Pet Care",
        pitches: [
            Pitch {
                title: "Your pet's morning routine",
                briefing: "Film your pet's morning, from waking up to breakfast with our \
                           grain-free food. Let your pet be the star.",
                genre_ids: &[genre::COMEDY, genre::MEMES, genre::LIFESTYLE],
            },
            Pitch {
                title: "New puppy checklist",
                briefing: "Share what you wish you had known before getting a puppy, with our \
                           starter kit in the shot.",
                genre_ids: &[genre::OTHER, genre::TUTORIALS, genre::LIFESTYLE],
            },
            Pitch {
                title: "The treat taste test",
                briefing: "Let your pet judge three of our new treat flavours. Their reaction is \
                           the review.",
                genre_ids: &[genre::COMEDY, genre::TIKTOK, genre::MEMES],
            },
        ],
    },
    Category {
        name: "Skincare",
        pitches: [
            Pitch {
                title: "Morning routine, real skin",
                briefing: "Walk through your morning skincare with our vitamin C serum and daily \
                           moisturiser. No filters; real skin texture is welcome.",
                genre_ids: &[genre::BEAUTY, genre::LIFESTYLE, genre::TUTORIALS],
            },
            Pitch {
                title: "SPF every day",
                briefing: "Show where our mineral sunscreen goes with you in an ordinary week: the \
                           run, the commute, the beach. Explain why you wear it daily.",
                genre_ids: &[genre::BEAUTY, genre::SPORTS, genre::TRAVEL],
            },
            Pitch {
                title: "Night routine reset",
                briefing: "Share your evening routine with our gentle cleanser and night cream, \
                           and how it helps you wind down.",
                genre_ids: &[genre::BEAUTY, genre::LIFESTYLE, genre::TIKTOK],
            },
        ],
    },
    Category {
        name: "Snacks",
        pitches: [
            Pitch {
                title: "Fuel your stream",
                briefing: "Keep a bag of our baked chips next to you while you stream or play, and \
                           let them make a natural cameo.",
                genre_ids: &[genre::GAMING, genre::STREAMING, genre::COMEDY],
            },
            Pitch {
                title: "Movie night box",
                briefing: "Host a movie night with our snack box, and tell us what you watched and \
                           which snack went first.",
                genre_ids: &[genre::FILM_CINEMA, genre::FOOD, genre::COMEDY],
            },
            Pitch {
                title: "Rate our weirdest flavour",
                briefing: "Taste our limited pickle-and-honey popcorn on camera and rate it out of \
                           ten. Honesty encouraged.",
                genre_ids: &[genre::MEMES, genre::TIKTOK, genre::FOOD],
            },
        ],
    },
    Category {
        name: "Tea",
        pitches: [
            Pitch {
                title: "Slow down with a cup",
                briefing: "Show a calm moment with our loose-leaf chamomile: reading, journaling, \
                           painting, whatever slows you down.",
                genre_ids: &[genre::BOOKS_WRITING, genre::ART, genre::LIFESTYLE],
            },
            Pitch {
                title: "Iced tea, three ways",
                briefing: "Make three iced teas with our fruit blends and share the recipes. \
                           Summer drinks, no syrup needed.",
                genre_ids: &[genre::FOOD, genre::TUTORIALS],
            },
            Pitch {
                title: "Matcha for the team",
                briefing: "Brew our matcha for your team or study group, and show who loved it and \
                           who didn't.",
                genre_ids: &[genre::BUSINESS, genre::BOOKS_WRITING, genre::COMEDY],
            },
        ],
    },
    Category {
        name: "Travel",
        pitches: [
            Pitch {
                title: "Weekend city escape",
                briefing: "Plan a weekend in a city you have never visited with our travel app, \
                           and film the sights, the food and the nights out like a trailer.",
                genre_ids: &[genre::TRAVEL, genre::FILM_CINEMA, genre::NIGHTLIFE],
            },
            Pitch {
                title: "Pack light challenge",
                briefing: "Fit a week of travel into our carry-on backpack and show how you packed \
                           it. Every item must earn its place.",
                genre_ids: &[genre::TRAVEL, genre::TUTORIALS, genre::TIKTOK],
            },
            Pitch {
                title: "Off-season gems",
                briefing: "Visit a popular destination out of season, booked through our app, and \
                           show what it's like without the crowds.",
                genre_ids: &[genre::TRAVEL, genre::FOOD, genre::ART],
            },
        ],
    },
];

/// Invented words, so that no advertiser carries a real company's name.
pub const BRANDS: [&str; 16] = [
    "Bluefinch",
    "Brightfern",
    "Cloudmint",
    "Copperwren",
    "Driftpine",
    "Emberlake",
    "Frostmoor",
    "Glowfield",
    "Hazelbrook",
    "Lumagrove",
    "Mistvale",
    "Northwind",
    "Oakhollow",
    "Quietbay",
    "Velvetfox",
    "Willowmere",
];

/// Who a campaign is for and what it pays.
pub struct Terms {
    pub platform: Platform,
    pub budget_cents: i64,
    pub target_cpm_cents: i64,
    pub size_groups: &'static [SizeGroup],
    pub sliders: Sliders,
    /// Empty for any.
    pub countries: &'static [&'static str],
    /// Empty for any.
    pub languages: &'static [&'static str],
}

/// A campaign's matching preferences, each 0, 25, 50, 75 or 100.
#[derive(Debug, Clone, Copy)]
pub struct Sliders {
    pub engagement: i16,
    pub quality: i16,
    pub reliability: i16,
}

/// Each advertiser's open campaign takes the next row, wrapping around; drafts pick rows at
/// random. The rows cover both platforms and every country and size, so the open campaigns
/// together reach most creators while each reaches a different part of them. With the default
/// rules, their target CPMs offer creators 76% to 136% of their usual rate.
#[rustfmt::skip]
pub static CAMPAIGN_TERMS: [Terms; 25] = [
    // Budgets in euros, target CPMs in cents; sliders are engagement, quality and reliability.
    //    platform   budget  CPM    sizes                  sliders        countries, languages
    terms(TikTok,    12_000, 2_400, &[Nano, Micro, Macro], [50, 50, 50],  &["DE", "AT"], &[]),
    terms(Instagram, 18_000, 2_200, &[Nano, Micro, Macro], [50, 75, 25],  &["GB", "US"], &[]),
    terms(TikTok,    50_000, 1_000, &[Macro, Mega],        [25, 75, 50],  &[], &[]),
    terms(Instagram, 4_000,  2_800, &[Nano, Micro],        [75, 25, 50],  &["DE"], &["de"]),
    terms(TikTok,    9_000,  2_000, &[Nano, Micro, Macro], [50, 50, 75],  &["GB", "US"], &["en"]),
    terms(Instagram, 2_000,  3_000, &[Nano, Micro],        [25, 50, 50],  &["AT"], &[]),
    terms(TikTok,    6_000,  1_800, &[Nano, Micro, Macro], [75, 25, 50],  &["US"], &["es"]),
    terms(Instagram, 25_000, 1_900, &[Nano, Micro, Macro], [50, 100, 50], &[], &["en"]),
    terms(TikTok,    3_000,  2_600, &[Nano, Micro, Macro], [100, 25, 25], &["DE"], &["tr"]),
    terms(Instagram, 40_000, 1_200, &[Micro, Macro],       [25, 75, 75],  &["DE", "AT", "GB"], &[]),
    terms(TikTok,    5_000,  2_200, &[Nano, Micro, Macro], [75, 50, 25],  &["GB"], &[]),
    terms(Instagram, 15_000, 1_600, &[Nano, Micro, Macro], [50, 50, 50],  &["US"], &[]),
    terms(TikTok,    30_000, 1_250, &[Micro, Macro],       [50, 50, 100], &[], &[]),
    terms(Instagram, 8_000,  2_000, &[Nano, Micro, Macro], [50, 75, 50],  &["DE", "AT"], &["de"]),
    terms(TikTok,    10_000, 1_800, &[Nano, Micro, Macro], [25, 50, 50],  &["DE", "AT"], &["en"]),
    terms(Instagram, 2_500,  3_000, &[Nano, Micro],        [75, 50, 25],  &["GB"], &[]),
    terms(TikTok,    45_000, 900,   &[Macro, Mega],        [0, 75, 50],   &["US"], &[]),
    terms(Instagram, 20_000, 1_100, &[Micro, Macro],       [50, 50, 50],  &[], &[]),
    terms(TikTok,    3_500,  2_500, &[Nano, Micro, Macro], [50, 25, 75],  &["AT"], &[]),
    terms(Instagram, 7_500,  1_800, &[Nano, Micro, Macro], [50, 50, 75],  &["DE"], &[]),
    terms(TikTok,    35_000, 1_300, &[Micro, Macro],       [75, 75, 50],  &["GB", "US"], &[]),
    terms(Instagram, 3_000,  2_400, &[Nano, Micro, Macro], [75, 25, 50],  &["US"], &["es"]),
    terms(TikTok,    16_000, 1_600, &[Nano, Micro, Macro], [50, 50, 50],  &["DE"], &[]),
    terms(Instagram, 48_000, 800,   &[Macro, Mega],        [25, 100, 75], &["GB", "US"], &["en"]),
    terms(TikTok,    6_500,  2_750, &[Nano, Micro],        [50, 25, 25],  &[], &["de", "en"]),
];

/// One row of [`CAMPAIGN_TERMS`], with arguments in the table's column order.
const fn terms(
    platform: Platform,
    budget_euros: i64,
    target_cpm_cents: i64,
    size_groups: &'static [SizeGroup],
    [engagement, quality, reliability]: [i16; 3],
    countries: &'static [&'static str],
    languages: &'static [&'static str],
) -> Terms {
    Terms {
        platform,
        budget_cents: budget_euros * 100,
        target_cpm_cents,
        size_groups,
        sliders: Sliders {
            engagement,
            quality,
            reliability,
        },
        countries,
        languages,
    }
}
