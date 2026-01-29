use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: usize,
    pub title: String,
    pub url: String,
    pub content: String,
}

pub struct SearchEngine {
    documents: Vec<Document>,
    index: HashMap<String, Vec<usize>>,
}

impl SearchEngine {
    pub fn new() -> Self {
        Self {
            documents: Vec::new(),
            index: HashMap::new(),
        }
    }

    pub fn add_document(&mut self, doc: Document) {
        let id = doc.id;
        let words: Vec<String> = doc.content
            .to_lowercase()
            .split_whitespace()
            .map(|s| s.to_string())
            .collect();

        for word in words {
            self.index.entry(word).or_insert_with(Vec::new).push(id);
        }

        self.documents.push(doc);
    }

    pub fn search(&self, query: &str) -> Vec<Document> {
        let query_lower = query.to_lowercase();
        let terms: Vec<&str> = query_lower.split_whitespace().collect();
        let mut doc_ids: Vec<usize> = Vec::new();

        for term in terms {
            if let Some(ids) = self.index.get(term) {
                doc_ids.extend(ids);
            }
        }

        doc_ids.sort();
        doc_ids.dedup();

        doc_ids.iter()
            .filter_map(|&id| self.documents.iter().find(|d| d.id == id))
            .cloned()
            .collect()
    }
}
