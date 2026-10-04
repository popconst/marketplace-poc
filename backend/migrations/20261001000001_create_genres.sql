-- What a creator's content is about: WePush's content categories, with the same ids and the
-- names translated from German.
CREATE TABLE genres (
    id   smallint PRIMARY KEY,
    name text NOT NULL UNIQUE
);

INSERT INTO genres (id, name) VALUES
    (1, 'Music'),
    (2, 'Gaming'),
    (3, 'Lifestyle'),
    (4, 'Travel'),
    (5, 'Sports'),
    (6, 'Food'),
    (7, 'Comedy'),
    (8, 'Beauty'),
    (9, 'Tech'),
    (10, 'Cars'),
    (11, 'Business'),
    (12, 'Film / Cinema'),
    (13, 'Art'),
    (14, 'Fitness'),
    (15, 'Streaming'),
    (16, 'TikTok'),
    (17, 'Memes'),
    (18, 'Nightlife'),
    (19, 'Books / Writing'),
    (20, 'Tutorials'),
    (21, 'Other');
