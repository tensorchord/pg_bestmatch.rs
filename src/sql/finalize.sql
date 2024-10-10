CREATE FUNCTION array_distinct(anyarray) RETURNS anyarray IMMUTABLE STRICT PARALLEL SAFE AS $fn$
  SELECT array_agg(DISTINCT x) FROM unnest($1) t(x);
$fn$ LANGUAGE SQL;

CREATE TABLE pg_bm25(
    -- classes
    attrelid regclass NOT NULL,
    attname NAME NOT NULL,
    matrelid regclass UNIQUE,
    indexrelid regclass UNIQUE,
    -- props
    b REAL NOT NULL,
    k1 REAL NOT NULL,
    tokenizer TEXT NOT NULL,
    model TEXT NOT NULL,
    -- cached
    words INT NOT NULL,
    docs INT NOT NULL,
    dims INT NOT NULL
);

CREATE FUNCTION bm25_create(tab regclass, col TEXT, mat TEXT, tokenizer TEXT DEFAULT 'hf', model TEXT DEFAULT 'google-bert/bert-base-uncased', b REAL DEFAULT 0.75, k1 REAL DEFAULT 1.2) RETURNS VOID AS $fn$
DECLARE
    test TEXT;
    ins_words INT;
    ins_docs INT;
    ins_dims INT;
BEGIN
    SELECT 'ok' INTO test FROM pg_catalog.pg_attribute WHERE attrelid = tab AND attname = col AND atttypid = 'text'::regtype;
    IF test != 'ok' THEN
        RAISE EXCEPTION 'This is no such table or no such column or column is not of type `text`.';
    END IF;
    EXECUTE format($$
        CREATE MATERIALIZED VIEW %s AS
            WITH
                inputs AS (SELECT bm_catalog.tokenize(%s, %L, %L) AS input FROM %s),
                tokens AS (SELECT unnest(input)::NAME COLLATE "C" AS token FROM inputs GROUP BY token ORDER BY token),
                compute_how_many_tokens AS (
                    SELECT unnest(input)::NAME COLLATE "C" AS t, count(*)::INT AS how_many_tokens
                    FROM inputs
                    GROUP BY t
                    ORDER BY t
                ),
                compute_token_in_how_many_inputs AS (
                    SELECT unnest(bm_catalog.array_distinct(input))::NAME COLLATE "C" AS t, count(*)::INT AS token_in_how_many_inputs
                    FROM inputs
                    GROUP BY t
                    ORDER BY t
                ),
                var_docs AS (SELECT count(*) AS docs FROM inputs)
            SELECT
                token,
                (row_number() OVER () - 1)::INT AS id,
                how_many_tokens,
                ln((docs + 1.0) / (token_in_how_many_inputs + 0.5))::REAL AS idf
            FROM tokens
            JOIN compute_how_many_tokens ON compute_how_many_tokens.t = token
            JOIN compute_token_in_how_many_inputs ON compute_token_in_how_many_inputs.t = token
            CROSS JOIN var_docs;
        CREATE INDEX %s_index ON %s(token);
    $$, mat, col, tokenizer, model, tab, mat, mat);
    EXECUTE format('SELECT sum(how_many_tokens) FROM %s', mat) INTO ins_words;
    EXECUTE format('SELECT count(%s) FROM %s', col, tab) INTO ins_docs;
    EXECUTE format('SELECT count(*) FROM %s', mat) INTO ins_dims;
    INSERT INTO bm_catalog.pg_bm25
    VALUES (tab, col, mat::regclass, (mat::text || '_index')::regclass, b, k1, tokenizer, model, ins_words, ins_docs, ins_dims);
END;
$fn$ LANGUAGE plpgsql;

CREATE FUNCTION bm25_refresh(mat regclass) RETURNS VOID AS $fn$
DECLARE
    tab regclass;
    col NAME;
    upd_words INT;
    upd_docs INT;
    upd_dims INT;
BEGIN
    SELECT attrelid, attname INTO tab, col FROM bm_catalog.pg_bm25 WHERE matrelid = mat;
    EXECUTE format('REFRESH MATERIALIZED VIEW %s', mat);
    EXECUTE format('SELECT sum(how_many_tokens) FROM %s', mat) INTO upd_words;
    EXECUTE format('SELECT count(%s) FROM %s', col, tab) INTO upd_docs;
    EXECUTE format('SELECT count(*) FROM %s', mat) INTO upd_dims;
    UPDATE bm_catalog.pg_bm25
    SET words = upd_words, docs = upd_docs, dims = upd_dims
    WHERE matrelid = mat;
END;
$fn$ LANGUAGE plpgsql;

CREATE FUNCTION bm25_drop(mat regclass) RETURNS VOID AS $fn$
BEGIN
    EXECUTE format('DROP MATERIALIZED VIEW %s', mat);
    DELETE FROM bm_catalog.pg_bm25
    WHERE matrelid = mat;
END;
$fn$ LANGUAGE plpgsql;

CREATE FUNCTION bm25_document_to_svector(mat regclass, t TEXT, style TEXT DEFAULT 'pgvecto.rs') RETURNS text STABLE STRICT PARALLEL SAFE AS $fn$
DECLARE
    idx regclass;
    p_b REAL;
    p_k1 REAL;
    p_words INT;
    p_docs INT;
    p_dims INT;
    p_tokenizer TEXT;
    p_model TEXT;
BEGIN
    SELECT indexrelid, b, k1, words, docs, dims, tokenizer, model INTO idx, p_b, p_k1, p_words, p_docs, p_dims, p_tokenizer, p_model FROM bm_catalog.pg_bm25 WHERE matrelid = mat;
    RETURN bm_catalog.bm25_document_to_svector_internal(mat::oid, idx::oid, p_b, p_k1, p_words, p_docs, p_dims, t, style, p_tokenizer, p_model);
END;
$fn$ LANGUAGE plpgsql;

CREATE FUNCTION bm25_query_to_svector(mat regclass, t TEXT, style TEXT DEFAULT 'pgvecto.rs') RETURNS text STABLE STRICT PARALLEL SAFE AS $fn$
DECLARE
    idx regclass;
    p_dims INT;
    p_tokenizer TEXT;
    p_model TEXT;
BEGIN
    SELECT indexrelid, dims, tokenizer, model INTO idx, p_dims, p_tokenizer, p_model FROM bm_catalog.pg_bm25 WHERE matrelid = mat;
    RETURN bm_catalog.bm25_query_to_svector_internal(mat::oid, idx::oid, p_dims, t, style, p_tokenizer, p_model);
END;
$fn$ LANGUAGE plpgsql;
