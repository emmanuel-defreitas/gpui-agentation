use std::{
    collections::{HashMap, VecDeque},
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use atomic_write_file::AtomicWriteFile;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::model::*;

/// Storage and validation errors shared by native UI and HTTP clients.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Requested session or annotation does not exist.
    #[error("feedback object not found")]
    NotFound,
    /// Client supplied invalid feedback.
    #[error("{0}")]
    Invalid(String),
    /// Storage could not be accessed.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// Stored feedback could not be decoded.
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Another thread failed while holding storage.
    #[error("feedback storage lock is poisoned")]
    Poisoned,
}

#[derive(Clone, Default, Deserialize, Serialize)]
struct Data {
    #[serde(default)]
    next_indexes: HashMap<String, u64>,
    #[serde(default)]
    toolbar_preferences: HashMap<String, ToolbarPreference>,
    sessions: Vec<Session>,
    events: VecDeque<Event>,
    revision: u64,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
pub(crate) struct ToolbarPreference {
    pub position: ToolbarPosition,
    pub at: Option<(f32, f32)>,
    #[serde(default)]
    pub last_position: ToolbarPosition,
}

/// Where Agentation's controls are displayed.
#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolbarPosition {
    /// Inline strip above application content.
    #[default]
    Banner,
    /// Draggable floating controls.
    Floating,
    /// Use the host's Debug menu without an embedded toolbar.
    None,
}

/// Shared feedback storage. Successful mutations are durable before being published.
#[derive(Clone)]
pub struct Store {
    data: Arc<Mutex<Data>>,
    path: Option<PathBuf>,
    pub(crate) events: broadcast::Sender<Event>,
}

impl Store {
    /// Load feedback from `path`, or create an empty store if it does not exist.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, Error> {
        let path = path.into();
        let mut data: Data = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Data::default(),
            Err(error) => return Err(error.into()),
        };
        for session in &mut data.sessions {
            let mut next = session
                .annotations
                .iter()
                .map(|a| a.index)
                .max()
                .unwrap_or(0)
                + 1;
            for annotation in &mut session.annotations {
                if annotation.index == 0 {
                    annotation.index = next;
                    next += 1;
                }
            }
            let saved = data.next_indexes.entry(session.id.clone()).or_insert(next);
            *saved = (*saved).max(next);
        }
        Ok(Self {
            data: Arc::new(Mutex::new(data)),
            path: Some(path),
            events: broadcast::channel(512).0,
        })
    }

    /// Create a transient store for tests or hosts that do not need restart persistence.
    pub fn memory() -> Self {
        Self {
            data: Arc::new(Mutex::new(Data::default())),
            path: None,
            events: broadcast::channel(512).0,
        }
    }

    /// List route sessions with their annotations.
    pub fn sessions(&self) -> Result<Vec<Session>, Error> {
        Ok(self
            .data
            .lock()
            .map_err(|_| Error::Poisoned)?
            .sessions
            .clone())
    }

    /// Read every annotation for an application, including resolved and dismissed notes.
    pub fn app_annotations(&self, app_name: &str) -> Result<Vec<Annotation>, Error> {
        Ok(self
            .sessions()?
            .into_iter()
            .filter(|session| session.app_name == app_name)
            .flat_map(|session| session.annotations)
            .collect())
    }

    /// Read one route's complete feedback history.
    pub fn session(&self, id: &str) -> Result<Session, Error> {
        self.sessions()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or(Error::NotFound)
    }

    /// Reuse a route's session, creating it if necessary.
    pub fn ensure_session(&self, app_name: &str, url: &str) -> Result<Session, Error> {
        text(app_name, 200)?;
        text(url, 2000)?;
        self.mutate(|data| {
            if let Some(session) = data
                .sessions
                .iter()
                .find(|s| s.app_name == app_name && s.url == url)
            {
                return Ok((session.clone(), None));
            }
            if data.sessions.len() >= 128 {
                return Err(Error::Invalid("session limit reached".into()));
            }
            let session = Session {
                id: Uuid::new_v4().to_string(),
                app_name: app_name.into(),
                url: url.into(),
                annotations: vec![],
            };
            data.sessions.push(session.clone());
            Ok((
                session.clone(),
                Some((
                    "session.created",
                    session.id.clone(),
                    serde_json::to_value(&session)?,
                )),
            ))
        })
    }

    /// Read a note by ID.
    pub fn annotation(&self, id: &str) -> Result<Annotation, Error> {
        self.sessions()?
            .into_iter()
            .flat_map(|s| s.annotations)
            .find(|a| a.id == id)
            .ok_or(Error::NotFound)
    }

    /// Add validated feedback to a session.
    pub fn add(&self, session_id: &str, input: AnnotationInput) -> Result<Annotation, Error> {
        validate(&input)?;
        self.mutate(|data| {
            let session = data
                .sessions
                .iter_mut()
                .find(|s| s.id == session_id)
                .ok_or(Error::NotFound)?;
            if session.annotations.len() >= 2000 {
                return Err(Error::Invalid("annotation limit reached".into()));
            }
            let next = data.next_indexes.entry(session.id.clone()).or_insert(1);
            let index = *next;
            *next += 1;
            let annotation = Annotation {
                index,
                id: Uuid::new_v4().to_string(),
                session_id: session.id.clone(),
                url: session.url.clone(),
                timestamp: now(),
                status: AnnotationStatus::Pending,
                kind: "feedback".into(),
                feedback: input,
                thread: vec![],
            };
            session.annotations.push(annotation.clone());
            Ok((
                annotation.clone(),
                Some((
                    "annotation.created",
                    session.id.clone(),
                    serde_json::to_value(&annotation)?,
                )),
            ))
        })
    }

    /// Number the next saved annotation will receive within this route.
    pub fn next_index(&self, session_id: &str) -> Result<u64, Error> {
        let data = self.data.lock().map_err(|_| Error::Poisoned)?;
        if !data.sessions.iter().any(|s| s.id == session_id) {
            return Err(Error::NotFound);
        }
        Ok(*data.next_indexes.get(session_id).unwrap_or(&1))
    }

    /// Update a comment or lifecycle status; unrelated fields are preserved.
    pub fn update(&self, id: &str, patch: AnnotationPatch) -> Result<Annotation, Error> {
        if let Some(comment) = &patch.comment {
            text(comment, 8000)?;
        }
        self.change(id, "annotation.updated", |a| {
            if let Some(comment) = patch.comment {
                if let Some(crate::AnnotationShape::Text { text, .. }) = &mut a.feedback.shape {
                    *text = comment.clone();
                }
                a.feedback.comment = comment;
            }
            if let Some(status) = patch.status {
                a.status = status;
            }
            Ok(())
        })
    }

    pub(crate) fn toolbar_preference(&self, app: &str) -> Result<ToolbarPreference, Error> {
        Ok(self
            .data
            .lock()
            .map_err(|_| Error::Poisoned)?
            .toolbar_preferences
            .get(app)
            .copied()
            .unwrap_or_default())
    }

    pub(crate) fn set_toolbar_preference(
        &self,
        app: &str,
        preference: ToolbarPreference,
    ) -> Result<(), Error> {
        if preference
            .at
            .is_some_and(|(x, y)| !x.is_finite() || !y.is_finite())
        {
            return Err(Error::Invalid("invalid toolbar position".into()));
        }
        self.mutate(|data| {
            data.toolbar_preferences.insert(app.to_string(), preference);
            Ok((
                (),
                Some((
                    "preferences.updated",
                    String::new(),
                    serde_json::to_value(preference)?,
                )),
            ))
        })
    }

    // Compare-and-swap a single note. Undo must never overwrite concurrent MCP
    // replies, status changes, or edits made from another window.
    pub(crate) fn replace_note(
        &self,
        expected: Option<&Annotation>,
        replacement: Option<&Annotation>,
    ) -> Result<(), Error> {
        let note = expected.or(replacement).ok_or(Error::NotFound)?;
        if let Some(next) = replacement {
            validate(&next.feedback)?;
            if next.id != note.id || next.session_id != note.session_id || next.index != note.index
            {
                return Err(Error::Invalid("annotation identity changed".into()));
            }
        }
        self.mutate(|data| {
            let session = data
                .sessions
                .iter_mut()
                .find(|s| s.id == note.session_id)
                .ok_or(Error::NotFound)?;
            let current = session.annotations.iter().find(|a| a.id == note.id);
            if current != expected {
                return Err(Error::Invalid(
                    "Annotation changed elsewhere; undo/redo was not applied".into(),
                ));
            }
            if current.is_none() && session.annotations.len() >= 2000 {
                return Err(Error::Invalid("annotation limit reached".into()));
            }
            session.annotations.retain(|a| a.id != note.id);
            let (kind, value) = if let Some(next) = replacement {
                session.annotations.push(next.clone());
                session.annotations.sort_by_key(|a| a.index);
                (
                    if expected.is_some() {
                        "annotation.updated"
                    } else {
                        "annotation.created"
                    },
                    serde_json::to_value(next)?,
                )
            } else {
                ("annotation.deleted", serde_json::json!({"id": note.id}))
            };
            Ok(((), Some((kind, session.id.clone(), value))))
        })
    }

    /// Append a thread reply from a human or agent.
    pub fn reply(&self, id: &str, role: &str, message: &str) -> Result<Annotation, Error> {
        text(role, 100)?;
        text(message, 8000)?;
        self.change(id, "thread.message", |a| {
            if a.thread.len() >= 500 {
                return Err(Error::Invalid("thread limit reached".into()));
            }
            a.thread.push(ThreadMessage {
                id: Uuid::new_v4().to_string(),
                role: role.into(),
                message: message.into(),
                timestamp: now(),
            });
            Ok(())
        })
    }

    fn change(
        &self,
        id: &str,
        kind: &'static str,
        change: impl FnOnce(&mut Annotation) -> Result<(), Error>,
    ) -> Result<Annotation, Error> {
        self.mutate(|data| {
            let a = data
                .sessions
                .iter_mut()
                .flat_map(|s| &mut s.annotations)
                .find(|a| a.id == id)
                .ok_or(Error::NotFound)?;
            change(a)?;
            Ok((
                a.clone(),
                Some((kind, a.session_id.clone(), serde_json::to_value(&a)?)),
            ))
        })
    }

    /// Delete a note permanently.
    pub fn delete(&self, id: &str) -> Result<(), Error> {
        self.mutate(|data| {
            let session = data
                .sessions
                .iter_mut()
                .find(|s| s.annotations.iter().any(|a| a.id == id))
                .ok_or(Error::NotFound)?;
            session.annotations.retain(|a| a.id != id);
            Ok((
                (),
                Some((
                    "annotation.deleted",
                    session.id.clone(),
                    serde_json::json!({"id": id}),
                )),
            ))
        })
    }

    /// Read only notes waiting for acknowledgement, optionally within one session.
    pub fn pending(&self, session_id: Option<&str>) -> Result<Vec<Annotation>, Error> {
        if let Some(id) = session_id {
            self.session(id)?;
        }
        Ok(self
            .sessions()?
            .into_iter()
            .filter(|s| session_id.is_none_or(|id| s.id == id))
            .flat_map(|s| s.annotations)
            .filter(|a| a.status == AnnotationStatus::Pending)
            .collect())
    }

    /// Current event cursor, also usable to detect UI updates without repainting constantly.
    pub fn revision(&self) -> Result<u64, Error> {
        Ok(self.data.lock().map_err(|_| Error::Poisoned)?.revision)
    }

    /// Replay retained events after `cursor`. An expired cursor requires a fresh session read.
    pub fn replay(&self, cursor: u64) -> Result<Vec<Event>, Error> {
        let data = self.data.lock().map_err(|_| Error::Poisoned)?;
        if cursor > data.revision
            || data
                .events
                .front()
                .is_some_and(|first| cursor.saturating_add(1) < first.id)
        {
            return Err(Error::Invalid(
                "event cursor expired; reload sessions and reconnect without Last-Event-ID".into(),
            ));
        }
        Ok(data
            .events
            .iter()
            .filter(|e| e.id > cursor)
            .cloned()
            .collect())
    }

    /// Markdown feedback for copying into a coding conversation.
    pub fn markdown(&self, session_id: &str) -> Result<String, Error> {
        let session = self.session(session_id)?;
        let mut output = format!(
            "# {} feedback\n\nRoute: {}\n",
            session.app_name, session.url
        );
        append_annotations(&mut output, &session.annotations)?;
        Ok(output)
    }

    /// Markdown for every route's annotations in an application, grouped by route.
    pub fn app_markdown(&self, app_name: &str) -> Result<String, Error> {
        let mut output = format!("# {app_name} feedback\n");
        for session in self
            .sessions()?
            .into_iter()
            .filter(|session| session.app_name == app_name && !session.annotations.is_empty())
        {
            output.push_str(&format!("\n## Route: {}\n", session.url));
            append_annotations(&mut output, &session.annotations)?;
        }
        Ok(output)
    }

    fn mutate<T>(
        &self,
        operation: impl FnOnce(
            &mut Data,
        )
            -> Result<(T, Option<(&'static str, String, serde_json::Value)>), Error>,
    ) -> Result<T, Error> {
        let mut guard = self.data.lock().map_err(|_| Error::Poisoned)?;
        let mut next = guard.clone();
        let (value, event) = operation(&mut next)?;
        let Some((kind, session_id, data)) = event else {
            return Ok(value);
        };
        next.revision += 1;
        let event = Event {
            id: next.revision,
            kind: kind.into(),
            session_id,
            data,
        };
        next.events.push_back(event.clone());
        while next.events.len() > 512 {
            next.events.pop_front();
        }
        if let Some(path) = &self.path {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            let mut file = AtomicWriteFile::open(path)?;
            file.write_all(&serde_json::to_vec(&next)?)?;
            file.commit()?;
        }
        *guard = next;
        let _ = self.events.send(event);
        Ok(value)
    }
}

fn append_annotations(output: &mut String, annotations: &[Annotation]) -> Result<(), Error> {
    for a in annotations {
        output.push_str(&format!(
            "\n{}. {} ({:?})\n   Target: `{}`\n   {}\n",
            a.index, a.feedback.element, a.status, a.feedback.element_path, a.feedback.comment
        ));
        if let Some(source) = &a.feedback.source_file {
            output.push_str(&format!("   Source: `{source}`\n"));
        }
        let b = a.feedback.bounding_box;
        output.push_str(&format!(
            "   Bounds: {}, {} · {} × {} logical px\n",
            b.x, b.y, b.width, b.height
        ));
        if let Some(shape) = &a.feedback.shape {
            output.push_str(&format!("   Shape: `{}`\n", serde_json::to_string(shape)?));
        }
        for message in &a.thread {
            output.push_str(&format!("   {}: {}\n", message.role, message.message));
        }
    }
    Ok(())
}

fn text(value: &str, max: usize) -> Result<(), Error> {
    if value.trim().is_empty() || value.chars().count() > max {
        return Err(Error::Invalid(format!(
            "text must contain 1–{max} characters"
        )));
    }
    Ok(())
}

fn validate(input: &AnnotationInput) -> Result<(), Error> {
    if let Some(shape) = &input.shape {
        if !shape.valid() {
            return Err(Error::Invalid("invalid annotation shape".into()));
        }
    }
    text(&input.comment, 8000)?;
    text(&input.element_path, 2000)?;
    text(&input.element, 500)?;
    if let Some(source) = &input.source_file {
        text(source, 2000)?;
    }
    if !input.x.is_finite()
        || !(0.0..=100.0).contains(&input.x)
        || !input.y.is_finite()
        || !input.bounding_box.valid()
    {
        return Err(Error::Invalid("invalid window coordinates".into()));
    }
    Ok(())
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(test)]
#[path = "store.test.rs"]
mod tests;
