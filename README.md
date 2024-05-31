# pg_bestmatch

## Usage

If you have a table,

```sql
DROP TABLE IF EXISTS t CASCADE;
CREATE TABLE t(doc TEXT NOT NULL);

INSERT INTO t(doc) VALUES
('this is a pen'),
('this is an apple'),
('this is a pen this is a pen'),
('this is an apple this is an apple'),
('those are pens'),
('those are apples'),
('those are pens those are pens'),
('those are apples those are apples');
```

If you want to search for documents by queries with BM25 algorithm, you could:

```sql
SELECT bm25_create('t', 'doc', 't_doc_bm25');

ALTER TABLE t ADD COLUMN embedding svector;
UPDATE t SET embedding = document_to_svector('t_doc_bm25', doc)::svector;
```

You could search for documents in this way:

```sql
SELECT doc FROM t ORDER BY embedding <#> query_to_svector('t_doc_bm25', 'i have an apple')::svector;
```

The statistics data used for BM25 is not updated on time. If you want to refresh it, you could:

```sql
SELECT bm25_refresh('t_doc_bm25');
```

If you do not use BM25 for a text column any more, you could delete the statistics data:

```sql
SELECT bm25_drop('t_doc_bm25');
```

In order to speed searching, you could create vector indexes.

```sql
-- you need to set type of column `doc` mannually everytime after `bm25_refresh` is executed
CREATE INDEX ON t USING vectors (doc svector_dot_ops);
```

## Benchmarking on LoCO

This extension is benchmarked on the [Stanford LoCo benchmark](https://hazyresearch.stanford.edu/blog/2024-01-11-m2-bert-retrieval).

```sh
wget https://huggingface.co/api/datasets/hazyresearch/LoCoV1-Documents/parquet/default/test/0.parquet -o documents.parquet
wget https://huggingface.co/api/datasets/hazyresearch/LoCoV1-Queries/parquet/default/test/0.parquet -o queries.parquet
```

```python
import pandas as pd
from sqlalchemy import create_engine
import numpy as np
from psycopg2.extensions import register_adapter, AsIs

def adapter_numpy_float64(numpy_float64):
    return AsIs(numpy_float64)

def adapter_numpy_int64(numpy_int64):
    return AsIs(numpy_int64)

def adapter_numpy_float32(numpy_float32):
    return AsIs(numpy_float32)

def adapter_numpy_int32(numpy_int32):
    return AsIs(numpy_int32)

def adapter_numpy_array(numpy_array):
    return AsIs(tuple(numpy_array))

register_adapter(np.float64, adapter_numpy_float64)
register_adapter(np.int64, adapter_numpy_int64)
register_adapter(np.float32, adapter_numpy_float32)
register_adapter(np.int32, adapter_numpy_int32)
register_adapter(np.ndarray, adapter_numpy_array)

db_url = "postgresql://localhost:5432/usamoi"
engine = create_engine(db_url)

def load_documents():
    df = pd.read_parquet("documents.parquet")
    df.to_sql("documents", engine, if_exists='replace', index=False)

def load_queries():
    df = pd.read_parquet("queries.parquet")
    df['answer_pids'] = df['answer_pids'].apply(lambda x: str(x[0]))    
    df.to_sql("queries", engine, if_exists='replace', index=False)

load_documents()
load_queries()
```

```sql
CREATE EXTENSION pg_bestmatch;

SELECT bm25_create('documents', 'passage', 'documents_passage_bm25', 0.75, 1.2);

ALTER TABLE documents ADD COLUMN embedding svector;

ALTER TABLE queries ADD COLUMN embedding svector;

UPDATE documents SET embedding = bm25_document_to_svector('documents_passage_bm25', passage)::svector;

UPDATE queries SET embedding = bm25_query_to_svector('documents_passage_bm25', query)::svector;

SELECT sum((array[answer_pids] = array(SELECT pid FROM documents WHERE queries.dataset = documents.dataset ORDER BY queries.embedding <#> documents.embedding LIMIT 1))::int) FROM queries;
```

Top 1 recall: 0.77411430049133695371.
