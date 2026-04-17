use crate::selection::Selection;
use crate::skin::SkinTexture;

#[derive(Clone, Debug)]
pub struct PixelChange {
    pub x: u32,
    pub y: u32,
    pub old_color: [u8; 4],
    pub new_color: [u8; 4],
}

#[derive(Clone)]
pub struct SelectionSnapshot {
    pub pixels: Vec<[u8; 4]>,
    pub w: u32,
    pub h: u32,
    pub x: i32,
    pub y: i32,
}

pub enum HistoryAction {
    PixelChanges {
        changes: Vec<PixelChange>,
        sel_before: Option<SelectionSnapshot>,
        sel_after: Option<SelectionSnapshot>,
    },
    SelectionTransform {
        before: SelectionSnapshot,
        after: SelectionSnapshot,
    },
}

pub struct HistoryEntry {
    pub description: String,
    pub action: HistoryAction,
}

impl HistoryEntry {
    pub fn from_changes(description: String, changes: Vec<PixelChange>) -> Self {
        Self {
            description,
            action: HistoryAction::PixelChanges {
                changes,
                sel_before: None,
                sel_after: None,
            },
        }
    }

    pub fn from_changes_with_selection(
        description: String,
        changes: Vec<PixelChange>,
        sel_before: Option<SelectionSnapshot>,
        sel_after: Option<SelectionSnapshot>,
    ) -> Self {
        Self {
            description,
            action: HistoryAction::PixelChanges {
                changes,
                sel_before,
                sel_after,
            },
        }
    }
}

pub struct History {
    undo_stack: Vec<HistoryEntry>,
    redo_stack: Vec<HistoryEntry>,
    max_entries: usize,
}

impl History {
    pub fn new() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_entries: 200,
        }
    }

    pub fn push(&mut self, entry: HistoryEntry) {
        match &entry.action {
            HistoryAction::PixelChanges { changes, .. } => {
                if changes.is_empty() {
                    return;
                }
            }
            HistoryAction::SelectionTransform { .. } => {
                // Always push transform entries
            }
        }
        self.redo_stack.clear();
        self.undo_stack.push(entry);
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.remove(0);
        }
    }

    pub fn undo(&mut self, skin: &mut SkinTexture, selection: &mut Selection) -> bool {
        if let Some(entry) = self.undo_stack.pop() {
            match &entry.action {
                HistoryAction::PixelChanges {
                    changes,
                    sel_before,
                    sel_after,
                } => {
                    for change in changes.iter().rev() {
                        skin.set_pixel(change.x, change.y, change.old_color);
                    }
                    // Restore selection state to before the operation
                    if let Some(snap) = sel_before {
                        selection.restore_snapshot(snap);
                    } else if sel_after.is_some() {
                        // sel_before=None, sel_after=Some → selection was inactive before
                        selection.deactivate();
                    }
                }
                HistoryAction::SelectionTransform { before, .. } => {
                    selection.restore_snapshot(before);
                }
            }
            self.redo_stack.push(entry);
            true
        } else {
            false
        }
    }

    pub fn redo(&mut self, skin: &mut SkinTexture, selection: &mut Selection) -> bool {
        if let Some(entry) = self.redo_stack.pop() {
            match &entry.action {
                HistoryAction::PixelChanges {
                    changes,
                    sel_after,
                    sel_before,
                } => {
                    for change in changes {
                        skin.set_pixel(change.x, change.y, change.new_color);
                    }
                    // Restore selection state to after the operation
                    if let Some(snap) = sel_after {
                        selection.restore_snapshot(snap);
                    } else if sel_after.is_none() && sel_before.is_some() {
                        // sel_after=None, sel_before=Some → selection became inactive
                        selection.deactivate();
                    }
                }
                HistoryAction::SelectionTransform { after, .. } => {
                    selection.restore_snapshot(after);
                }
            }
            self.undo_stack.push(entry);
            true
        } else {
            false
        }
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn undo_count(&self) -> usize {
        self.undo_stack.len()
    }

    pub fn redo_count(&self) -> usize {
        self.redo_stack.len()
    }

    pub fn undo_descriptions(&self) -> Vec<&str> {
        self.undo_stack.iter().map(|e| e.description.as_str()).collect()
    }

    pub fn redo_descriptions(&self) -> Vec<&str> {
        self.redo_stack.iter().rev().map(|e| e.description.as_str()).collect()
    }

    pub fn undo_to(&mut self, target_undo_count: usize, skin: &mut SkinTexture, selection: &mut Selection) {
        while self.undo_stack.len() > target_undo_count {
            if !self.undo(skin, selection) {
                break;
            }
        }
    }

    pub fn redo_to(&mut self, target_undo_count: usize, skin: &mut SkinTexture, selection: &mut Selection) {
        while self.undo_stack.len() < target_undo_count {
            if !self.redo(skin, selection) {
                break;
            }
        }
    }
}
