//! The genre ids, as the migration `20261001000001_create_genres.sql` inserts them, and which
//! genres go together.

pub const MUSIC: i16 = 1;
pub const GAMING: i16 = 2;
pub const LIFESTYLE: i16 = 3;
pub const TRAVEL: i16 = 4;
pub const SPORTS: i16 = 5;
pub const FOOD: i16 = 6;
pub const COMEDY: i16 = 7;
pub const BEAUTY: i16 = 8;
pub const TECH: i16 = 9;
pub const CARS: i16 = 10;
pub const BUSINESS: i16 = 11;
pub const FILM_CINEMA: i16 = 12;
pub const ART: i16 = 13;
pub const FITNESS: i16 = 14;
pub const STREAMING: i16 = 15;
pub const TIKTOK: i16 = 16;
pub const MEMES: i16 = 17;
pub const NIGHTLIFE: i16 = 18;
pub const BOOKS_WRITING: i16 = 19;
pub const TUTORIALS: i16 = 20;
pub const OTHER: i16 = 21;

/// Each genre's neighbours: genres that accounts often cover alongside it. An account's other
/// genres are drawn from its main genre's neighbours.
pub static NEIGHBOURS: [(i16, &[i16]); 21] = [
    (LIFESTYLE, &[BEAUTY, FITNESS, FOOD, TRAVEL, TUTORIALS]),
    (BEAUTY, &[LIFESTYLE, TUTORIALS, TIKTOK]),
    (FITNESS, &[SPORTS, LIFESTYLE, FOOD]),
    (SPORTS, &[FITNESS, CARS, GAMING]),
    (FOOD, &[LIFESTYLE, TRAVEL, TUTORIALS]),
    (TRAVEL, &[LIFESTYLE, FOOD, FILM_CINEMA]),
    (GAMING, &[STREAMING, TECH, MEMES, COMEDY]),
    (STREAMING, &[GAMING, MUSIC, COMEDY]),
    (TECH, &[GAMING, BUSINESS, TUTORIALS, CARS]),
    (CARS, &[TECH, SPORTS]),
    (BUSINESS, &[TECH, BOOKS_WRITING]),
    (MUSIC, &[NIGHTLIFE, TIKTOK, STREAMING, ART]),
    (NIGHTLIFE, &[MUSIC, LIFESTYLE]),
    (COMEDY, &[MEMES, TIKTOK, FILM_CINEMA]),
    (MEMES, &[COMEDY, TIKTOK, GAMING]),
    (TIKTOK, &[MEMES, COMEDY, MUSIC, BEAUTY]),
    (FILM_CINEMA, &[ART, COMEDY, BOOKS_WRITING]),
    (ART, &[FILM_CINEMA, MUSIC, BOOKS_WRITING]),
    (BOOKS_WRITING, &[ART, BUSINESS, FILM_CINEMA]),
    (TUTORIALS, &[TECH, BEAUTY, FOOD]),
    (OTHER, &[]),
];
