//! The schemas of a pocket's records and of a pulled batch, and the column
//! indexes every step reads by.

use std::sync::{Arc, LazyLock};

use arrow_schema::{DataType, Field, Schema, SchemaRef};

/// The record key, `Utf8`.
pub(super) const RECORD_KEY: usize = 0;
/// The latest rank, `Int64`.
pub(super) const RANK: usize = 1;
/// The latest payload, `Binary`.
pub(super) const PAYLOAD: usize = 2;
/// The record's position, `UInt64`.
pub(super) const POSITION: usize = 3;
/// The record's status, `UInt8`: [`QUEUED`], [`HELD`] or [`DEFERRED`].
pub(super) const STATUS: usize = 4;
/// The held version's rank, `Int64`, null unless held.
pub(super) const HELD_RANK: usize = 5;
/// The held version's payload, `Binary`, null unless held.
pub(super) const HELD_PAYLOAD: usize = 6;

/// `status` of a queued record.
pub(super) const QUEUED: u8 = 0;
/// `status` of a held record.
pub(super) const HELD: u8 = 1;
/// `status` of a deferred record.
pub(super) const DEFERRED: u8 = 2;

/// One pocket's records: `record_key`, `rank`, `payload`, `position`, `status`,
/// `held_rank` and `held_payload`, in that order.
pub(super) static RECORDS: LazyLock<SchemaRef> = LazyLock::new(|| {
    Arc::new(Schema::new(vec![
        Field::new("record_key", DataType::Utf8, false),
        Field::new("rank", DataType::Int64, false),
        Field::new("payload", DataType::Binary, false),
        Field::new("position", DataType::UInt64, false),
        Field::new("status", DataType::UInt8, false),
        Field::new("held_rank", DataType::Int64, true),
        Field::new("held_payload", DataType::Binary, true),
    ]))
});

/// A pulled batch: the held versions' `record_key`, `rank` and `payload`, oldest
/// first. The same three columns, in the same order, as the records schema
/// starts with.
pub(super) static BATCH: LazyLock<SchemaRef> = LazyLock::new(|| {
    Arc::new(Schema::new(vec![
        Field::new("record_key", DataType::Utf8, false),
        Field::new("rank", DataType::Int64, false),
        Field::new("payload", DataType::Binary, false),
    ]))
});
