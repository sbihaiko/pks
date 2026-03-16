# Database Indexing Strategies

## Overview

A database index is a data structure that accelerates row retrieval at the cost of additional storage and write overhead. Choosing the right index type, columns, and cardinality characteristics is one of the highest-leverage performance optimizations available to application developers.

## Index Types

### B-Tree Indexes

B-tree indexes are the default in most relational databases. They support equality, range, prefix, and sort operations efficiently. The tree stays balanced through splits and merges, maintaining O(log n) lookup regardless of table size. They are the right choice for most columns queried with =, <, >, BETWEEN, or LIKE 'prefix%'.

### Hash Indexes

Hash indexes provide O(1) exact equality lookups but cannot support range queries or sorting. PostgreSQL builds hash indexes in-memory for some query plans even when a B-tree is defined. Standalone hash indexes are best for primary key lookups in OLTP workloads with very high query rates.

### Full-Text Search Indexes

Full-text indexes tokenize strings, apply stemming and stop-word removal, and build an inverted index mapping tokens to document identifiers with term frequency scores. BM25 ranking weighs term frequency against inverse document frequency to produce relevance scores. Full-text search indexes are essential for searching document content, unlike B-tree indexes which match exact strings.

### Composite Indexes

A composite index covers multiple columns. Column order matters: the index is useful for queries that filter on a prefix of the index columns. An index on (status, created_at) helps queries filtering by status, or by both status and created_at, but not by created_at alone.

## Index Maintenance

Indexes must be rebuilt or reindexed periodically as table bloat accumulates dead tuples. EXPLAIN ANALYZE output reveals whether the query planner selects the intended index. Unused indexes identified through pg_stat_user_indexes should be dropped; they consume write I/O for no query benefit.

## Covering Indexes

An index that includes all columns referenced in a query satisfies the query from the index alone, eliminating heap fetches. This index-only scan dramatically reduces I/O for frequent read-heavy queries.
