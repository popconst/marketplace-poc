-- A campaign with its targeting lists, each sorted, as the API shows it. An empty list means no
-- restriction. Postgres expands c.* when the view is created, so a column added to campaigns
-- later needs the view created again.
CREATE VIEW campaign_details AS
SELECT
    c.*,
    ARRAY(SELECT country_code FROM campaign_countries
          WHERE campaign_id = c.id ORDER BY country_code) AS country_codes,
    ARRAY(SELECT language_code FROM campaign_languages
          WHERE campaign_id = c.id ORDER BY language_code) AS language_codes,
    ARRAY(SELECT genre_id FROM campaign_genres
          WHERE campaign_id = c.id ORDER BY genre_id) AS genre_ids
FROM campaigns c;
