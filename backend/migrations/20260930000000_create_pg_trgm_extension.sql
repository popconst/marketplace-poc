-- Trigram indexes, which make substring search (ILIKE '%lena%') fast on creator names and
-- account handles.
CREATE EXTENSION IF NOT EXISTS pg_trgm;
