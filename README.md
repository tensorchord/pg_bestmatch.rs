# pg_bestmatch

```sql
DROP TABLE IF EXISTS t CASCADE;
CREATE TABLE t(doc TEXT NOT NULL, embedding svector);

INSERT INTO t(doc) VALUES
('this is a pen'),
('this is an apple'),
('this is a pen this is a pen'),
('this is an apple this is an apple'),
('those are pens'),
('those are apples'),
('those are pens those are pens'),
('those are apples those are apples');

DROP EXTENSION IF EXISTS pg_bestmatch;
CREATE EXTENSION pg_bestmatch CASCADE;

SELECT bestmatch_create('t', 'doc', 't_doc_bm', 0.75, 1.2);

SELECT * FROM t_doc_bm;

UPDATE t SET embedding = document_to_svector('t_doc_bm', doc)::svector;

SELECT * FROM t;

SELECT doc FROM t ORDER BY embedding <#> query_to_svector('t_doc_bm', 'i have an apple')::svector;

INSERT INTO t(doc) VALUES
('i have a pen'),
('i have an apple'),
('i have a pen i have a pen'),
('i have an apple i have an apple'),
('you have pens'),
('you have apples'),
('you have pens you have pens'),
('you have apples you have apples');

SELECT bestmatch_refresh('t_doc_bm');

SELECT doc FROM t ORDER BY embedding <#> query_to_svector('t_doc_bm', 'i have an apple')::svector;

SELECT bestmatch_drop('t_doc_bm');
```
