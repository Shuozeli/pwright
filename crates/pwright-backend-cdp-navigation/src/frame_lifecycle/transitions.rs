use super::*;

impl FrameLifecycleStore {
    pub(super) fn transition(&mut self, input: LifecycleInput) -> Result<(), LifecycleError> {
        match input {
            LifecycleInput::Attach { frame, parent } => self.attach(frame, parent),
            LifecycleInput::Commit {
                frame,
                document,
                request,
            } => self.commit(frame, document, request),
            LifecycleInput::DocumentEvent {
                frame,
                document,
                event,
            } => {
                let Some(index) = self.frame_index(frame) else {
                    return Ok(());
                };
                let row = self.frames[index].as_mut().unwrap();
                if row.snapshot.document != Some(document) {
                    return Ok(());
                }
                let bit = match event {
                    DocumentLifecycle::DomContentLoaded => &mut row.snapshot.dom_content_loaded,
                    DocumentLifecycle::Load => &mut row.snapshot.load,
                };
                if !*bit {
                    *bit = true;
                    self.recalculate();
                }
                Ok(())
            }
            LifecycleInput::RequestStarted {
                frame,
                request,
                favicon,
            } => self.request_started(frame, request, favicon),
            LifecycleInput::RequestFinished { request } => self.request_finished(request),
            LifecycleInput::Detach { frame } => {
                self.remove_subtree(frame, true);
                self.recalculate();
                Ok(())
            }
        }
    }

    fn attach(&mut self, frame: ObjectId, parent: Option<ObjectId>) -> Result<(), LifecycleError> {
        if frame.get() == 0 {
            return Err(LifecycleError::InvalidIdentity);
        }
        if let Some(index) = self.frame_index(frame) {
            return if self.frames[index].as_ref().unwrap().parent == parent {
                Ok(())
            } else {
                Err(LifecycleError::ConflictingIdentity)
            };
        }
        let depth = match parent {
            Some(parent) => {
                self.frames[self
                    .frame_index(parent)
                    .ok_or(LifecycleError::MissingParent)?]
                .as_ref()
                .unwrap()
                .depth
                    + 1
            }
            None if self.frames.iter().any(Option::is_some) => {
                return Err(LifecycleError::ConflictingIdentity);
            }
            None => 0,
        };
        let index = self
            .frames
            .iter()
            .position(Option::is_none)
            .ok_or(LifecycleError::FrameCapacity)?;
        let deadline = self.idle_deadline()?;
        self.frames[index] = Some(FrameState {
            id: frame,
            parent,
            depth,
            snapshot: LifecycleSnapshot::initial(),
            requests: 0,
            self_idle: false,
            deadline: Some(deadline),
            retiring: false,
        });
        Ok(())
    }

    fn commit(
        &mut self,
        frame: ObjectId,
        document: DocumentGeneration,
        request: Option<ObjectId>,
    ) -> Result<(), LifecycleError> {
        if document.0 == 0 {
            return Err(LifecycleError::InvalidIdentity);
        }
        let Some(index) = self.frame_index(frame) else {
            return Ok(());
        };
        if self.frames[index]
            .as_ref()
            .unwrap()
            .snapshot
            .document
            .is_some_and(|current| current >= document)
        {
            return Ok(());
        }
        if self
            .requests
            .iter()
            .flatten()
            .any(|row| Some(row.id) == request && row.frame != frame)
        {
            return Err(LifecycleError::ConflictingIdentity);
        }
        let deadline = self.idle_deadline()?;
        self.remove_subtree(frame, false);
        let mut active = 0;
        for slot in &mut self.requests {
            if let Some(row) = slot.as_ref().filter(|row| row.frame == frame) {
                if Some(row.id) == request {
                    active += 1;
                } else {
                    *slot = None;
                }
            }
        }
        let row = self.frames[index].as_mut().unwrap();
        row.snapshot = LifecycleSnapshot {
            document: Some(document),
            ..LifecycleSnapshot::initial()
        };
        row.requests = active;
        row.self_idle = false;
        row.deadline = (active == 0).then_some(deadline);
        self.recalculate();
        Ok(())
    }

    fn request_started(
        &mut self,
        frame: ObjectId,
        request: ObjectId,
        favicon: bool,
    ) -> Result<(), LifecycleError> {
        if favicon {
            return Ok(());
        }
        if request.get() == 0 {
            return Err(LifecycleError::InvalidIdentity);
        }
        let Some(index) = self.frame_index(frame) else {
            return Ok(());
        };
        if let Some(existing) = self.requests.iter().flatten().find(|row| row.id == request) {
            return if existing.frame == frame {
                Ok(())
            } else {
                Err(LifecycleError::ConflictingIdentity)
            };
        }
        let slot = self
            .requests
            .iter()
            .position(Option::is_none)
            .ok_or(LifecycleError::RequestCapacity)?;
        self.requests[slot] = Some(RequestState { id: request, frame });
        let row = self.frames[index].as_mut().unwrap();
        row.requests += 1;
        if row.requests == 1 {
            row.deadline = None;
            row.self_idle = false;
        }
        // Request start stops the timer; it does not itself recalculate the
        // already-published lifecycle bits in the pinned server.
        Ok(())
    }

    fn request_finished(&mut self, request: ObjectId) -> Result<(), LifecycleError> {
        let Some(slot) = self
            .requests
            .iter()
            .position(|row| row.as_ref().is_some_and(|row| row.id == request))
        else {
            return Ok(());
        };
        let deadline = self.idle_deadline()?;
        let request = self.requests[slot].take().unwrap();
        if let Some(index) = self.frame_index(request.frame) {
            let row = self.frames[index].as_mut().unwrap();
            row.requests -= 1;
            if row.requests == 0 && !row.snapshot.network_idle {
                row.deadline = Some(deadline);
            }
        }
        Ok(())
    }

    fn remove_subtree(&mut self, frame: ObjectId, include_root: bool) {
        let max_depth = self
            .frames
            .iter()
            .flatten()
            .map(|row| row.depth)
            .max()
            .unwrap_or(0);
        // Mark in parent-first order before deleting anything. The scratch bit
        // lives in each admitted slot: O(F^2 + R*F), no ancestry recursion or
        // temporary vector, and no dangling-parent lookup after partial removal.
        for depth in 0..=max_depth {
            for index in 0..self.frames.len() {
                let Some(row) = self.frames[index].as_ref().filter(|row| row.depth == depth) else {
                    continue;
                };
                let retiring = row.id == frame
                    || row
                        .parent
                        .and_then(|parent| self.frame_index(parent))
                        .is_some_and(|parent| self.frames[parent].as_ref().unwrap().retiring);
                self.frames[index].as_mut().unwrap().retiring = retiring;
            }
        }
        for index in 0..self.requests.len() {
            if self.requests[index].as_ref().is_some_and(|request| {
                (include_root || request.frame != frame)
                    && self
                        .frame_index(request.frame)
                        .is_some_and(|frame| self.frames[frame].as_ref().unwrap().retiring)
            }) {
                self.requests[index] = None;
            }
        }
        for slot in &mut self.frames {
            if let Some(row) = slot {
                if row.retiring && (include_root || row.id != frame) {
                    *slot = None;
                } else {
                    row.retiring = false;
                }
            }
        }
    }
}
