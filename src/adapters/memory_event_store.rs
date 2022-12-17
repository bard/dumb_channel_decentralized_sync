use crate::app::{Bookmark, BookmarkId, BookmarkQuery, EventStore};
use std::sync::Mutex;

#[cfg(test)]
use mock_instant::Instant;

#[cfg(not(test))]
use std::time::Instant;

#[derive(std::fmt::Debug, PartialEq, Clone)]
pub struct DomainEventEnvelope {
    time: Instant,
    payload: DomainEvent,
}

#[derive(std::fmt::Debug, PartialEq, Clone)]
pub enum DomainEvent {
    BookmarkCreated {
        id: BookmarkId,
        url: String,
        title: String,
    },
    BookmarkDeleted {
        id: BookmarkId,
    },
}

pub struct MemoryEventStore {
    events: Mutex<Vec<DomainEventEnvelope>>,
}

impl MemoryEventStore {
    pub fn new() -> Self {
        let events: Mutex<Vec<DomainEventEnvelope>> = Mutex::new(vec![]);
        Self { events }
    }
}

impl EventStore for MemoryEventStore {
    fn read_bookmark(&self, query: &BookmarkQuery) -> Option<Bookmark> {
        match self.events.lock() {
            Ok(lock) => lock.iter().fold(None, |acc, event| match &event.payload {
                DomainEvent::BookmarkCreated { id, url, title } => {
                    if id == &query.id {
                        Some(Bookmark {
                            id: id.clone(),
                            title: title.clone(),
                            url: url.clone(),
                        })
                    } else {
                        acc
                    }
                }
                DomainEvent::BookmarkDeleted { id } => {
                    if id == &query.id {
                        None
                    } else {
                        acc
                    }
                }
            }),
            _ => panic!(),
        }
    }

    fn save_bookmark(&self, bookmark: &Bookmark) -> Result<BookmarkId, ()> {
        match self.events.lock() {
            Ok(mut lock) => {
                lock.push(DomainEventEnvelope {
                    time: Instant::now(),
                    payload: DomainEvent::BookmarkCreated {
                        id: bookmark.id.clone(),
                        url: bookmark.url.clone(),
                        title: bookmark.title.clone(),
                    },
                });
                Ok(bookmark.id.clone())
            }
            _ => panic!(),
        }
    }

    fn delete_bookmark(&self, query: &BookmarkQuery) -> () {
        match self.events.lock() {
            Ok(mut lock) => {
                lock.push(DomainEventEnvelope {
                    time: Instant::now(),
                    payload: DomainEvent::BookmarkDeleted {
                        id: query.id.clone(),
                    },
                });
            }
            _ => panic!(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mock_instant::{Instant, MockClock};
    use std::time::Duration;

    #[test]
    fn test_creation_causes_event_to_be_added_to_log() {
        let creation_time = Instant::now();
        let store = MemoryEventStore::new();
        store
            .save_bookmark(&Bookmark {
                id: String::from("123"),
                url: String::from("https://example.com"),
                title: String::from("Example"),
            })
            .unwrap();

        MockClock::advance(Duration::from_secs(10));

        let events = store.events.lock().unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            DomainEventEnvelope {
                time: creation_time,
                payload: DomainEvent::BookmarkCreated {
                    id: String::from("123"),
                    url: String::from("https://example.com"),
                    title: String::from("Example"),
                }
            }
        );
    }
}
