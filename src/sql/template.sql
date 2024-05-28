CREATE MATERIALIZED VIEW {mat} AS
    WITH
        inputs AS (SELECT bm_catalog.tokenize({col}) AS input FROM {tab}),
        tokens AS (SELECT unnest(input)::NAME COLLATE "C" AS token FROM inputs GROUP BY token ORDER BY token),
        compute_how_many_tokens AS (
            SELECT unnest(input)::NAME COLLATE "C" AS t, count(*)::INT AS how_many_tokens
            FROM inputs
            GROUP BY t
            ORDER BY t
        ),
        compute_token_in_how_many_inputs AS (
            SELECT unnest(array_distinct(input))::NAME COLLATE "C" AS t, count(*)::INT AS token_in_how_many_inputs
            FROM inputs
            GROUP BY t
            ORDER BY t
        )
    SELECT
        token, (row_number() OVER () - 1)::INT AS id, how_many_tokens, token_in_how_many_inputs
    FROM tokens
    JOIN compute_how_many_tokens ON compute_how_many_tokens.t = token
    JOIN compute_token_in_how_many_inputs ON compute_token_in_how_many_inputs.t = token;

CREATE INDEX {mat}_index ON {mat}(token);
