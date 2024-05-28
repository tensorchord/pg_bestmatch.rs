CREATE FUNCTION array_distinct(anyarray) RETURNS anyarray AS $f$
  SELECT array_agg(DISTINCT x) FROM unnest($1) t(x);
$f$ LANGUAGE SQL IMMUTABLE;

CREATE TABLE bestmatch(
    -- classes
    attrelid regclass NOT NULL,
    attname NAME NOT NULL,
    matrelid regclass UNIQUE,
    indexrelid regclass UNIQUE,
    -- props
    b REAL NOT NULL,
    k1 REAL NOT NULL,
    -- cached
    words INT NOT NULL,
    docs INT NOT NULL,
    dims INT NOT NULL
);

CREATE OR REPLACE FUNCTION bestmatch_create(tab regclass, col NAME, mat NAME, b REAL, k1 REAL) RETURNS VOID AS $$
DECLARE
    result TEXT;
    ins_words INT;
    ins_docs INT;
    ins_dims INT;
BEGIN
    SELECT 'ok' INTO result FROM pg_catalog.pg_attribute WHERE attrelid = tab AND attname = col AND atttypid = 'text'::regtype;
    IF result != 'ok' THEN
        RAISE EXCEPTION 'This is no such table or no such column or column is not of type `text`.';
    END IF;
    EXECUTE bm_catalog.bestmatch_template(tab::TEXT, col::TEXT, mat::TEXT);
    EXECUTE format('SELECT sum(how_many_tokens) FROM %s', mat) INTO ins_words;
    EXECUTE format('SELECT count(%s) FROM %s', col, tab) INTO ins_docs;
    EXECUTE format('SELECT count(*) FROM %s', mat) INTO ins_dims;
    INSERT INTO bm_catalog.bestmatch
    VALUES (tab, col, mat::regclass, (mat::text || '_index')::regclass, b, k1, ins_words, ins_docs, ins_dims);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION bestmatch_refresh(mat regclass) RETURNS VOID AS $$
DECLARE
    tab regclass;
    col NAME;
    upd_words INT;
    upd_docs INT;
    upd_dims INT;
BEGIN
    SELECT attrelid, attname INTO tab, col FROM bm_catalog.bestmatch WHERE matrelid = mat;
    EXECUTE format('REFRESH MATERIALIZED VIEW %s', mat);
    EXECUTE format('SELECT sum(how_many_tokens) FROM %s', mat) INTO upd_words;
    EXECUTE format('SELECT count(%s) FROM %s', col, tab) INTO upd_docs;
    EXECUTE format('SELECT count(*) FROM %s', mat) INTO upd_dims;
    UPDATE bm_catalog.bestmatch
    SET words = upd_words, docs = upd_docs, dims = upd_dims
    WHERE matrelid = mat;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION bestmatch_drop(mat regclass) RETURNS VOID AS $$
BEGIN
    EXECUTE format('DROP MATERIALIZED VIEW %s', mat);
    DELETE FROM bm_catalog.bestmatch
    WHERE matrelid = mat;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION document_to_svector(mat regclass, t TEXT) RETURNS text AS $$
DECLARE
    idx regclass;
    p_b REAL;
    p_k1 REAL;
    p_words INT;
    p_docs INT;
    p_dims INT;
BEGIN
    SELECT indexrelid, b, k1, words, docs, dims
    INTO idx, p_b, p_k1, p_words, p_docs, p_dims
    FROM bm_catalog.bestmatch
    WHERE matrelid = mat;
    RETURN bm_catalog.document_to_svector_internal(mat::oid, idx::oid, p_b, p_k1, p_words, p_docs, p_dims, t);
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE FUNCTION query_to_svector(mat regclass, t TEXT) RETURNS text AS $$
DECLARE
    idx regclass;
    p_b REAL;
    p_k1 REAL;
    p_words INT;
    p_docs INT;
    p_dims INT;
BEGIN
    SELECT indexrelid, b, k1, words, docs, dims
    INTO idx, p_b, p_k1, p_words, p_docs, p_dims
    FROM bm_catalog.bestmatch
    WHERE matrelid = mat;
    RETURN bm_catalog.query_to_svector_internal(mat::oid, idx::oid, p_b, p_k1, p_words, p_docs, p_dims, t);
END;
$$ LANGUAGE plpgsql;
