use std::{collections::HashMap, sync::Arc};

use arrow_array::{
    Array, ArrayRef, FixedSizeListArray, Float32Array, Float64Array, RecordBatch,
    RecordBatchIterator, StringArray,
};
use arrow_schema::{DataType, Field, Schema};
use futures::TryStreamExt;
use lancedb::{
    DistanceType, Table,
    connection::Connection,
    index::Index,
    query::{ExecutableQuery, QueryBase},
};

use crate::vector_store::{SearchHit, StoredChunk, VectorStore, VectorStoreResult};

const DISTANCE_COLUMN: &str = "_distance";
const VECTOR_COLUMN: &str = "vector";

pub struct LanceDbStore {
    connection: Connection,
    table_name: String,
    vector_size: usize,
}

impl LanceDbStore {
    pub async fn new(uri: &str, table_name: &str, vector_size: usize) -> VectorStoreResult<Self> {
        let connection = lancedb::connect(uri).execute().await?;
        let store = Self {
            connection,
            table_name: table_name.to_string(),
            vector_size,
        };
        store.ensure_ready().await?;
        Ok(store)
    }

    fn schema(&self) -> Arc<Schema> {
        let vector_field = Field::new("item", DataType::Float32, false);
        Arc::new(Schema::new(vec![
            Field::new("point_id", DataType::Utf8, false),
            Field::new("doc_id", DataType::Utf8, false),
            Field::new("chunk_id", DataType::Utf8, false),
            Field::new("path", DataType::Utf8, false),
            Field::new("title_path", DataType::Utf8, false),
            Field::new("section", DataType::Utf8, false),
            Field::new("text", DataType::Utf8, false),
            Field::new("hash", DataType::Utf8, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(Arc::new(vector_field), self.vector_size as i32),
                false,
            ),
        ]))
    }

    async fn create_table_if_absent(&self) -> VectorStoreResult<()> {
        let names = self.connection.table_names().execute().await?;
        if names.iter().any(|name| name == &self.table_name) {
            return Ok(());
        }

        self.connection
            .create_empty_table(&self.table_name, self.schema())
            .execute()
            .await?;

        Ok(())
    }

    async fn open_table(&self) -> VectorStoreResult<Table> {
        let table = self
            .connection
            .open_table(&self.table_name)
            .execute()
            .await?;
        Ok(table)
    }

    fn column_as_string<'a>(
        &self,
        batch: &'a RecordBatch,
        name: &str,
    ) -> VectorStoreResult<&'a StringArray> {
        let idx = batch
            .schema_ref()
            .index_of(name)
            .map_err(|e| crate::vector_store::VectorStoreError::InvalidPayload(e.to_string()))?;
        batch
            .column(idx)
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| {
                crate::vector_store::VectorStoreError::InvalidPayload(format!(
                    "column `{name}` is not StringArray"
                ))
            })
    }

    fn distance_for_row(&self, batch: &RecordBatch, row: usize) -> VectorStoreResult<f32> {
        let idx = batch
            .schema_ref()
            .index_of(DISTANCE_COLUMN)
            .map_err(|e| crate::vector_store::VectorStoreError::InvalidPayload(e.to_string()))?;

        let array = batch.column(idx);
        if let Some(values) = array.as_any().downcast_ref::<Float32Array>() {
            return Ok(values.value(row));
        }
        if let Some(values) = array.as_any().downcast_ref::<Float64Array>() {
            return Ok(values.value(row) as f32);
        }

        Err(crate::vector_store::VectorStoreError::InvalidPayload(
            "distance column type is neither Float32 nor Float64".to_string(),
        ))
    }

    fn distance_to_score(distance: f32) -> f32 {
        (1.0 - (distance / 2.0)).clamp(0.0, 1.0)
    }

    fn escape_sql_literal(value: &str) -> String {
        value.replace('\'', "''")
    }

    async fn try_ensure_vector_index(&self, table: &Table) -> VectorStoreResult<()> {
        let row_count = table.count_rows(None).await?;
        if row_count == 0 {
            return Ok(());
        }

        let has_vector_index = table
            .list_indices()
            .await?
            .iter()
            .any(|index| index.columns.iter().any(|column| column == VECTOR_COLUMN));

        if has_vector_index {
            return Ok(());
        }

        table
            .create_index(&[VECTOR_COLUMN], Index::Auto)
            .execute()
            .await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl VectorStore for LanceDbStore {
    async fn ensure_ready(&self) -> VectorStoreResult<()> {
        self.create_table_if_absent().await?;

        let table = self.open_table().await?;
        if let Err(err) = self.try_ensure_vector_index(&table).await {
            tracing::warn!(
                table = %self.table_name,
                error = %err,
                "failed to create or reuse vector index"
            );
        }

        Ok(())
    }

    async fn search(
        &self,
        query_vector: Vec<f32>,
        top_k: u64,
    ) -> VectorStoreResult<Vec<SearchHit>> {
        let table = self.open_table().await?;
        let stream = table
            .query()
            .nearest_to(query_vector)?
            .distance_type(DistanceType::Cosine)
            .limit(top_k as usize)
            .execute()
            .await?;

        let batches: Vec<RecordBatch> = stream.try_collect().await?;
        let mut results = Vec::new();

        for batch in batches {
            let doc_id = self.column_as_string(&batch, "doc_id")?;
            let path = self.column_as_string(&batch, "path")?;
            let title_path = self.column_as_string(&batch, "title_path")?;
            let section = self.column_as_string(&batch, "section")?;
            let text = self.column_as_string(&batch, "text")?;

            for row in 0..batch.num_rows() {
                let distance = self.distance_for_row(&batch, row)?;
                results.push(SearchHit {
                    doc_id: doc_id.value(row).to_string(),
                    path: path.value(row).to_string(),
                    title_path: title_path.value(row).to_string(),
                    section: section.value(row).to_string(),
                    snippet: text.value(row).to_string(),
                    score: Self::distance_to_score(distance),
                });
            }
        }

        Ok(results)
    }

    async fn upsert_chunks(&self, chunks: Vec<StoredChunk>) -> VectorStoreResult<()> {
        if chunks.is_empty() {
            return Ok(());
        }

        let mut vectors = Vec::with_capacity(chunks.len() * self.vector_size);
        for chunk in &chunks {
            if chunk.vector.len() != self.vector_size {
                return Err(crate::vector_store::VectorStoreError::InvalidPayload(
                    format!(
                        "vector size mismatch: got {}, expected {}",
                        chunk.vector.len(),
                        self.vector_size
                    ),
                ));
            }
            vectors.extend_from_slice(&chunk.vector);
        }

        let schema = self.schema();

        let point_id = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.point_id.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;
        let doc_id = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.doc_id.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;
        let chunk_id = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.chunk_id.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;
        let path = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.path.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;
        let title_path = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.title_path.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;
        let section = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.section.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;
        let text = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.text.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;
        let hash = Arc::new(StringArray::from(
            chunks
                .iter()
                .map(|chunk| chunk.hash.as_str())
                .collect::<Vec<_>>(),
        )) as ArrayRef;

        let vector_values = Arc::new(Float32Array::from(vectors)) as ArrayRef;
        let vector = Arc::new(FixedSizeListArray::new(
            Arc::new(Field::new("item", DataType::Float32, false)),
            self.vector_size as i32,
            vector_values,
            None,
        )) as ArrayRef;

        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                point_id, doc_id, chunk_id, path, title_path, section, text, hash, vector,
            ],
        )?;
        let reader = RecordBatchIterator::new(vec![Ok(batch)].into_iter(), schema);
        let table = self.open_table().await?;

        table.add(Box::new(reader)).execute().await?;
        if let Err(err) = self.try_ensure_vector_index(&table).await {
            tracing::warn!(
                table = %self.table_name,
                error = %err,
                "failed to create or reuse vector index"
            );
        }
        Ok(())
    }

    async fn delete_by_doc_id(&self, doc_id: &str) -> VectorStoreResult<()> {
        let table = self.open_table().await?;
        let escaped = Self::escape_sql_literal(doc_id);
        let predicate = format!("doc_id = '{escaped}'");
        table.delete(&predicate).await?;
        Ok(())
    }

    async fn list_doc_hashes(&self) -> VectorStoreResult<HashMap<String, String>> {
        let table = self.open_table().await?;
        let stream = table.query().execute().await?;
        let batches: Vec<RecordBatch> = stream.try_collect().await?;
        let mut map = HashMap::new();

        for batch in batches {
            let doc_id = self.column_as_string(&batch, "doc_id")?;
            let hash = self.column_as_string(&batch, "hash")?;
            for row in 0..batch.num_rows() {
                map.insert(doc_id.value(row).to_string(), hash.value(row).to_string());
            }
        }

        Ok(map)
    }

    async fn count(&self) -> VectorStoreResult<usize> {
        let table = self.open_table().await?;
        let count = table.count_rows(None).await?;
        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::LanceDbStore;

    #[test]
    fn distance_to_score_converts_and_clamps() {
        assert_eq!(LanceDbStore::distance_to_score(0.0), 1.0);
        assert_eq!(LanceDbStore::distance_to_score(2.0), 0.0);
        assert_eq!(LanceDbStore::distance_to_score(1.0), 0.5);
        assert_eq!(LanceDbStore::distance_to_score(-1.0), 1.0);
        assert_eq!(LanceDbStore::distance_to_score(3.0), 0.0);
    }
}
