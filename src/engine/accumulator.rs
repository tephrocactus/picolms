use crate::schema::DomainField;
use arrow::array::ArrayBuilder;
use arrow::array::BooleanBuilder;
use arrow::array::Float64Builder;
use arrow::array::Int64Builder;
use arrow::array::ListBuilder;
use arrow::array::RecordBatch;
use arrow::array::StringBuilder;
use arrow::datatypes::DataType;
use arrow::datatypes::Schema;
use arrow::datatypes::SchemaRef;
use parquet::arrow::ArrowWriter;
use parquet::basic::Compression;
use parquet::errors::ParquetError;
use parquet::file::properties::WriterProperties;
use parquet::format::FileMetaData;
use serde_json::Value as JsonValue;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::select;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::error;
use tokio::sync::mpsc::Receiver;
use tokio::sync::mpsc::Sender;
use tokio::sync::oneshot;
use tokio::task::spawn_blocking;
use tokio::time::interval;
use tokio::time::Interval;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use uuid::Uuid;

type FieldName = String;
type Builders = Vec<Box<dyn ArrayBuilder>>;
type FailedRows = Vec<JsonValue>;
type BlockId = Uuid;

#[derive(Debug, Error)]
pub enum Error {
    #[error("source is expected to be an object")]
    ObjectExpected,
    #[error("homogeneous array expected: {0}")]
    HomogeneousArrayExpected(FieldName),
    #[error("missing field: {0}")]
    MissingField(FieldName),
    #[error("type missmatch: {0}")]
    TypeMissmatch(FieldName),
    #[error("parquet error: {0}")]
    Parquet(#[from] ParquetError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

pub struct Input {
    rows: Rows,
    tx: oneshot::Sender<Result<FailedRows, Error>>,
}
pub enum Rows {
    Json(Vec<JsonValue>),
}

pub struct Accumulator {
    tx: Sender<Input>,
}

struct ParquetBuilder {
    schema: Arc<Schema>,
    fields: Vec<Box<dyn ArrayBuilder>>,
}

impl Accumulator {
    const MAX_ROWS: usize = 8192;

    pub fn new(schema: Arc<Schema>, tt: &TaskTracker, dir: PathBuf) -> Self {
        let (tx, rx) = channel(1);
        tt.spawn(Self::worker(rx, schema, dir));
        Self { tx }
    }

    pub async fn add_rows(&self, rows: Rows) -> Result<FailedRows, Error> {
        let (tx, rx) = oneshot::channel();
        self.tx.send(Input { rows, tx }).await.unwrap();
        rx.await.unwrap()
    }

    async fn worker(mut rx: Receiver<Input>, schema: Arc<Schema>, dir: PathBuf) {
        let mut ticker = interval(Duration::from_secs(10));
        let mut rows_count: usize = 0;
        let mut builders = schema
            .fields()
            .iter()
            .map(|f| f.builder())
            .collect::<Builders>();

        if rows_count >= Self::MAX_ROWS {
            Self::flush(schema, &mut builders, &dir);
            ticker.reset();
            rows_count = 0;
        }

        loop {
            select! {
                _ = ticker.tick() => {
                    Self::flush(schema.clone(), &mut builders, &dir).await;
                }

                input = rx.recv() => {
                    match input {
                        None => return,
                        Some(input) => Self::_add_rows(&mut builders, input)
                    }
                }
            }
        }
    }

    async fn flush(schema: SchemaRef, builders: &mut Builders, dir: &PathBuf) -> Result<(), Error> {
        let block_id = Uuid::now_v7();
        let batch = Self::get_batch(schema, builders);
        let file_path = dir.join(block_id.to_string());
        let props = WriterProperties::builder()
            .set_created_by(String::new())
            .set_compression(Compression::LZ4_RAW)
            .build();

        spawn_blocking(move || {
            let file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(file_path)?;
            let mut writer = ArrowWriter::try_new(&file, batch.schema(), Some(props)).unwrap();
            writer.write(&batch)?;
            writer.close()?;
            file.sync_all()
        })
        .await
        .unwrap()?;

        Ok(())
    }

    fn get_batch(schema: SchemaRef, builders: &mut Builders) -> RecordBatch {
        RecordBatch::try_new(schema, builders.iter_mut().map(|v| v.finish()).collect()).unwrap()
    }

    fn _add_rows(builders: &mut Builders, input: Input) {}

    fn add_rows_json(builders: &mut Builders, values: Vec<JsonValue>, rows: &mut usize) {}

    fn add_row_json(
        schema: &Schema,
        builders: &mut Builders,
        value: &JsonValue,
    ) -> Result<(), Error> {
        if !value.is_object() {
            Err(Error::ObjectExpected)?;
        }

        for (f, b) in schema.fields().iter().zip(builders.iter_mut()) {
            let v = match value.get(f.name()) {
                Some(v) => v,
                None if !f.is_nullable() => Err(Error::MissingField(f.name().clone()))?,
                None => return Ok(f.append_null(b)),
            };

            let b = b.as_any_mut();

            match f.data_type() {
                DataType::Utf8 => b.downcast_mut::<StringBuilder>().unwrap().append_value(
                    v.as_str()
                        .ok_or_else(|| Error::TypeMissmatch(f.name().clone()))?,
                ),

                DataType::Int64 => b.downcast_mut::<Int64Builder>().unwrap().append_value(
                    v.as_i64()
                        .ok_or_else(|| Error::TypeMissmatch(f.name().clone()))?,
                ),

                DataType::Float64 => b.downcast_mut::<Float64Builder>().unwrap().append_value(
                    v.as_f64()
                        .ok_or_else(|| Error::TypeMissmatch(f.name().clone()))?,
                ),

                DataType::Boolean => b.downcast_mut::<BooleanBuilder>().unwrap().append_value(
                    v.as_bool()
                        .ok_or_else(|| Error::TypeMissmatch(f.name().clone()))?,
                ),

                DataType::List(nested) => {
                    let array = v
                        .as_array()
                        .ok_or_else(|| Error::TypeMissmatch(f.name().clone()))?;

                    match nested.data_type() {
                        DataType::Utf8 => {
                            if !array.iter().all(|v| v.is_string()) {
                                Err(Error::HomogeneousArrayExpected(f.name().clone()))?;
                            }

                            b.downcast_mut::<ListBuilder<StringBuilder>>()
                                .unwrap()
                                .append_value(array.iter().map(|v| Some(v.as_str().unwrap())))
                        }

                        DataType::Int64 => {
                            if !array.iter().all(|v| v.is_i64()) {
                                Err(Error::HomogeneousArrayExpected(f.name().clone()))?;
                            }

                            b.downcast_mut::<ListBuilder<Int64Builder>>()
                                .unwrap()
                                .append_value(array.iter().map(|v| Some(v.as_i64().unwrap())))
                        }

                        DataType::Float64 => {
                            if !array.iter().all(|v| v.is_f64()) {
                                Err(Error::HomogeneousArrayExpected(f.name().clone()))?;
                            }

                            b.downcast_mut::<ListBuilder<Float64Builder>>()
                                .unwrap()
                                .append_value(array.iter().map(|v| Some(v.as_f64().unwrap())))
                        }

                        DataType::Boolean => {
                            if !array.iter().all(|v| v.is_boolean()) {
                                Err(Error::HomogeneousArrayExpected(f.name().clone()))?;
                            }

                            b.downcast_mut::<ListBuilder<BooleanBuilder>>()
                                .unwrap()
                                .append_value(array.iter().map(|v| Some(v.as_bool().unwrap())))
                        }

                        _ => unreachable!(),
                    }
                }

                _ => unreachable!(),
            }
        }

        Ok(())
    }
}
