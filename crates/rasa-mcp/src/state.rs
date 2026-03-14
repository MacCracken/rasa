use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use rasa_core::Document;
use uuid::Uuid;

/// Session state for the MCP server — manages open documents.
pub struct SessionState {
    documents: Mutex<HashMap<Uuid, Document>>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            documents: Mutex::new(HashMap::new()),
        }
    }

    /// Create a new document and return its ID.
    pub fn create_document(&self, name: &str, width: u32, height: u32) -> Uuid {
        let doc = Document::new(name, width, height);
        let id = doc.id;
        self.documents.lock().unwrap_or_else(|e| e.into_inner()).insert(id, doc);
        id
    }

    /// Open an image file as a new document.
    pub fn open_image(&self, path: &PathBuf) -> Result<Uuid, rasa_core::error::RasaError> {
        let doc = rasa_storage::import::import_image(path)?;
        let id = doc.id;
        self.documents.lock().unwrap_or_else(|e| e.into_inner()).insert(id, doc);
        Ok(id)
    }

    /// Access a document by ID for reading.
    pub fn with_doc<R>(
        &self,
        id: Uuid,
        f: impl FnOnce(&Document) -> R,
    ) -> Result<R, rasa_core::error::RasaError> {
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        let doc = docs
            .get(&id)
            .ok_or(rasa_core::error::RasaError::Other(format!(
                "document not found: {id}"
            )))?;
        Ok(f(doc))
    }

    /// Access a document by ID for mutation.
    pub fn with_doc_mut<R>(
        &self,
        id: Uuid,
        f: impl FnOnce(&mut Document) -> R,
    ) -> Result<R, rasa_core::error::RasaError> {
        let mut docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        let doc = docs
            .get_mut(&id)
            .ok_or(rasa_core::error::RasaError::Other(format!(
                "document not found: {id}"
            )))?;
        Ok(f(doc))
    }

    /// List all open document IDs with names.
    pub fn list_documents(&self) -> Vec<(Uuid, String, u32, u32)> {
        let docs = self.documents.lock().unwrap_or_else(|e| e.into_inner());
        docs.values()
            .map(|d| (d.id, d.name.clone(), d.size.width, d.size.height))
            .collect()
    }

    /// Close a document.
    pub fn close_document(&self, id: Uuid) -> bool {
        self.documents.lock().unwrap_or_else(|e| e.into_inner()).remove(&id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_and_list() {
        let state = SessionState::new();
        let id = state.create_document("Test", 100, 100);
        let docs = state.list_documents();
        assert_eq!(docs.len(), 1);
        assert_eq!(docs[0].0, id);
        assert_eq!(docs[0].1, "Test");
    }

    #[test]
    fn with_doc_reads() {
        let state = SessionState::new();
        let id = state.create_document("Canvas", 200, 150);
        let name = state.with_doc(id, |d| d.name.clone()).unwrap();
        assert_eq!(name, "Canvas");
    }

    #[test]
    fn with_doc_mut_modifies() {
        let state = SessionState::new();
        let id = state.create_document("Canvas", 100, 100);
        state
            .with_doc_mut(id, |d| {
                d.add_layer(rasa_core::layer::Layer::new_raster("New", 100, 100));
            })
            .unwrap();
        let count = state.with_doc(id, |d| d.layers.len()).unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn close_document() {
        let state = SessionState::new();
        let id = state.create_document("Temp", 10, 10);
        assert!(state.close_document(id));
        assert!(!state.close_document(id));
    }

    #[test]
    fn missing_doc_errors() {
        let state = SessionState::new();
        let result = state.with_doc(Uuid::new_v4(), |_| {});
        assert!(result.is_err());
    }
}
