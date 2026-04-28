use crate::domain::{AnalysisWindow, StoredEvent, VideoChunk};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Default)]
pub struct InMemoryRepository {
    chunks_by_seq: BTreeMap<u64, VideoChunk>,
    processed_window_starts: BTreeMap<u64, Uuid>,
    windows: BTreeMap<Uuid, AnalysisWindow>,
    events: Vec<StoredEvent>,
}

impl InMemoryRepository {
    pub fn insert_chunk(&mut self, chunk: VideoChunk) {
        self.chunks_by_seq.insert(chunk.chunk_seq, chunk);
    }

    pub fn build_completed_window(&mut self, start_seq: u64) -> Option<AnalysisWindow> {
        if self.processed_window_starts.contains_key(&start_seq) {
            return None;
        }

        let chunks = (start_seq..start_seq + 4)
            .map(|seq| self.chunks_by_seq.get(&seq).cloned())
            .collect::<Option<Vec<_>>>()?;

        let first = chunks.first()?;
        let last = chunks.last()?;

        let window = AnalysisWindow {
            id: Uuid::new_v4(),
            source_chunk_start: first.chunk_seq,
            source_chunk_end: last.chunk_seq,
            start_ts: first.start_ts,
            end_ts: last.end_ts,
            chunk_storage_uris: chunks.into_iter().map(|chunk| chunk.storage_uri).collect(),
        };

        self.processed_window_starts.insert(start_seq, window.id);
        self.windows.insert(window.id, window.clone());

        Some(window)
    }

    pub fn get_window(&self, id: Uuid) -> Option<AnalysisWindow> {
        self.windows.get(&id).cloned()
    }

    pub fn upsert_events(&mut self, mut new_events: Vec<StoredEvent>) -> Vec<StoredEvent> {
        let mut persisted = Vec::with_capacity(new_events.len());

        for event in new_events.drain(..) {
            if let Some(last) = self.events.last_mut() {
                if can_merge(last, &event) {
                    merge_into(last, &event);
                    persisted.push(last.clone());
                    continue;
                }
            }

            self.events.push(event.clone());
            persisted.push(event);
        }

        persisted
    }

    pub fn events(&self) -> &[StoredEvent] {
        &self.events
    }
}

fn can_merge(left: &StoredEvent, right: &StoredEvent) -> bool {
    left.source_chunk_end + 1 == right.source_chunk_start
        && left.event_type == right.event_type
        && left.location == right.location
        && left.activity == right.activity
}

fn merge_into(target: &mut StoredEvent, incoming: &StoredEvent) {
    target.source_chunk_end = incoming.source_chunk_end;
    target.end_ts = max_ts(target.end_ts, incoming.end_ts);
    target.importance = target.importance.max(incoming.importance);
    target.confidence = target.confidence.max(incoming.confidence);

    for object in &incoming.objects {
        if !target.objects.contains(object) && target.objects.len() < 8 {
            target.objects.push(object.clone());
        }
    }

    for trigger in &incoming.trigger_candidates {
        if !target.trigger_candidates.contains(trigger) {
            target.trigger_candidates.push(trigger.clone());
        }
    }

    target.description = incoming.description.clone();
    target.search_text = incoming.search_text.clone();
}

fn max_ts(left: DateTime<Utc>, right: DateTime<Utc>) -> DateTime<Utc> {
    if left >= right {
        left
    } else {
        right
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::VideoChunk;
    use chrono::TimeZone;

    #[test]
    fn builds_window_only_when_four_consecutive_chunks_exist() {
        let mut repo = InMemoryRepository::default();
        for seq in 10..13 {
            repo.insert_chunk(VideoChunk {
                id: Uuid::new_v4(),
                chunk_seq: seq,
                start_ts: Utc.with_ymd_and_hms(2026, 4, 27, 12, seq as u32, 0).unwrap(),
                end_ts: Utc.with_ymd_and_hms(2026, 4, 27, 12, seq as u32, 30).unwrap(),
                storage_uri: format!("gs://test/{seq}.mp4"),
            });
        }

        assert!(repo.build_completed_window(10).is_none());

        repo.insert_chunk(VideoChunk {
            id: Uuid::new_v4(),
            chunk_seq: 13,
            start_ts: Utc.with_ymd_and_hms(2026, 4, 27, 12, 13, 0).unwrap(),
            end_ts: Utc.with_ymd_and_hms(2026, 4, 27, 12, 13, 30).unwrap(),
            storage_uri: "gs://test/13.mp4".to_string(),
        });

        let window = repo.build_completed_window(10).expect("window should exist");
        assert_eq!(window.source_chunk_start, 10);
        assert_eq!(window.source_chunk_end, 13);
        assert_eq!(window.chunk_storage_uris.len(), 4);
    }
}
