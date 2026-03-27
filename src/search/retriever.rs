use std::collections::HashMap;

use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Field, Schema, TantivyDocument, OwnedValue, STORED, STRING, TEXT};
use tantivy::{Index, IndexReader, IndexWriter, ReloadPolicy, Term};

use crate::indexer::chunker::Chunk;

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub file_path: String,
    pub heading_hierarchy: Vec<String>,
    pub chunk_text: String,
    pub chunk_hash: String,
    pub score: f32,
    pub repo_id: String,
}

/// Metadata stored per chunk hash for snapshot filtering and RRF metadata fallback.
#[derive(Debug, Clone)]
pub struct ChunkMeta {
    pub repo_id: String,
    pub file_path: String,
    pub heading_hierarchy: Vec<String>,
    pub chunk_index: usize,
    pub chunk_hash: String,
    pub text: String,
}

pub trait SearchBackend {
    fn add_chunk(&mut self, chunk: &Chunk) -> tantivy::Result<()>;
    fn remove_chunks_for_file(&mut self, repo_id: &str, file_path: &str) -> tantivy::Result<()>;
    fn remove_chunks_for_repo(&mut self, repo_id: &str) -> tantivy::Result<()>;
    fn search(&self, query: &str, top_k: usize, repo_filter: Option<&[String]>) -> tantivy::Result<Vec<SearchResult>>;
    fn commit(&mut self) -> tantivy::Result<()>;
    fn add_chunk_with_vector(&mut self, chunk: &Chunk, _vector: Vec<f32>) -> tantivy::Result<()> {
        self.add_chunk(chunk)
    }
}

struct SchemaFields {
    repo_id: Field,
    file_path: Field,
    heading_hierarchy: Field,
    chunk_text: Field,
    chunk_hash: Field,
}

pub struct TantivyBackend {
    index: Index,
    writer: IndexWriter,
    reader: IndexReader,
    fields: SchemaFields,
    pub vectors: HashMap<String, Vec<f32>>,
    pub chunk_meta: HashMap<String, ChunkMeta>,
}

fn build_schema() -> (Schema, SchemaFields) {
    let mut builder = Schema::builder();
    let repo_id = builder.add_text_field("repo_id", TEXT | STORED);
    let file_path = builder.add_text_field("file_path", STRING | STORED);
    let heading_hierarchy = builder.add_text_field("heading_hierarchy", TEXT | STORED);
    let chunk_text = builder.add_text_field("chunk_text", TEXT | STORED);
    let chunk_hash = builder.add_text_field("chunk_hash", STRING | STORED);
    (builder.build(), SchemaFields { repo_id, file_path, heading_hierarchy, chunk_text, chunk_hash })
}

fn extract_str(doc: &TantivyDocument, field: Field) -> &str {
    doc.get_first(field).and_then(|v| match v {
        OwnedValue::Str(s) => Some(s.as_str()),
        _ => None,
    }).unwrap_or("")
}

impl TantivyBackend {
    pub fn new_in_memory() -> tantivy::Result<Self> {
        let (schema, fields) = build_schema();
        let index = Index::create_in_ram(schema);
        let writer = index.writer(50_000_000)?;
        let reader = index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into()?;
        Ok(Self { index, writer, reader, fields, vectors: HashMap::new(), chunk_meta: HashMap::new() })
    }

    pub fn new_on_disk(path: &std::path::Path) -> tantivy::Result<Self> {
        let (schema, fields) = build_schema();
        let index = Index::open_or_create(tantivy::directory::MmapDirectory::open(path)?, schema)?;
        let writer = index.writer(50_000_000)?;
        let reader = index.reader_builder().reload_policy(ReloadPolicy::Manual).try_into()?;
        Ok(Self { index, writer, reader, fields, vectors: HashMap::new(), chunk_meta: HashMap::new() })
    }
}

fn repo_passes_filter(repo_id: &str, repo_filter: Option<&[String]>) -> bool {
    repo_filter.is_none_or(|filter| filter.iter().any(|r| r == repo_id))
}

impl SearchBackend for TantivyBackend {
    fn add_chunk(&mut self, chunk: &Chunk) -> tantivy::Result<()> {
        let mut doc = TantivyDocument::default();
        doc.add_text(self.fields.repo_id, &chunk.repo_id);
        doc.add_text(self.fields.file_path, &chunk.file_path);
        doc.add_text(self.fields.heading_hierarchy, chunk.heading_hierarchy.join(" > "));
        doc.add_text(self.fields.chunk_text, &chunk.text);
        doc.add_text(self.fields.chunk_hash, &chunk.chunk_hash);
        self.writer.add_document(doc)?;
        self.chunk_meta.insert(chunk.chunk_hash.clone(), ChunkMeta {
            repo_id: chunk.repo_id.clone(),
            file_path: chunk.file_path.clone(),
            heading_hierarchy: chunk.heading_hierarchy.clone(),
            chunk_index: chunk.chunk_index,
            chunk_hash: chunk.chunk_hash.clone(),
            text: chunk.text.clone(),
        });
        Ok(())
    }

    fn add_chunk_with_vector(&mut self, chunk: &Chunk, vector: Vec<f32>) -> tantivy::Result<()> {
        self.add_chunk(chunk)?;
        self.vectors.insert(chunk.chunk_hash.clone(), vector);
        Ok(())
    }

    fn remove_chunks_for_file(&mut self, _repo_id: &str, file_path: &str) -> tantivy::Result<()> {
        self.writer.delete_term(Term::from_field_text(self.fields.file_path, file_path));
        Ok(())
    }

    fn remove_chunks_for_repo(&mut self, repo_id: &str) -> tantivy::Result<()> {
        self.writer.delete_term(Term::from_field_text(self.fields.repo_id, repo_id));
        Ok(())
    }

    fn search(&self, query: &str, top_k: usize, repo_filter: Option<&[String]>) -> tantivy::Result<Vec<SearchResult>> {
        let searcher = self.reader.searcher();
        let query_parser = QueryParser::for_index(&self.index, vec![self.fields.chunk_text, self.fields.heading_hierarchy]);
        let top_docs = searcher.search(&query_parser.parse_query(query)?, &TopDocs::with_limit(top_k * 10))?;
        let mut results = Vec::new();
        for (score, doc_address) in top_docs {
            let doc: TantivyDocument = searcher.doc(doc_address)?;
            let repo_id = extract_str(&doc, self.fields.repo_id).to_string();
            if !repo_passes_filter(&repo_id, repo_filter) { continue; }
            results.push(SearchResult {
                file_path: extract_str(&doc, self.fields.file_path).to_string(),
                heading_hierarchy: vec![extract_str(&doc, self.fields.heading_hierarchy).to_string()],
                chunk_text: extract_str(&doc, self.fields.chunk_text).to_string(),
                chunk_hash: extract_str(&doc, self.fields.chunk_hash).to_string(),
                score, repo_id,
            });
            if results.len() >= top_k { break; }
        }
        Ok(results)
    }

    fn commit(&mut self) -> tantivy::Result<()> {
        self.writer.commit()?;
        self.reader.reload()?;
        Ok(())
    }
}
