use std::collections::HashMap;

use iron_core::NodeId;
use taffy::{AvailableSpace, Size, Style, TaffyTree};

/// A node's computed layout, relative to its parent's border box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Resolved border widths: top, right, bottom, left.
    pub border: [f32; 4],
}

/// The layout backend (DESIGN.md §5.3). Taffy is the default; a Yoga backend can
/// implement this and run the same fixtures (`tests/layout/*.json`).
///
/// Taffy's `Style` doubles as the backend-neutral description of CSS layout
/// properties; another backend translates it.
pub trait LayoutEngine {
    fn create_node(&mut self, id: NodeId, style: &Style);
    fn remove_node(&mut self, id: NodeId);
    fn set_style(&mut self, id: NodeId, style: &Style);
    fn set_children(&mut self, parent: NodeId, children: &[NodeId]);
    /// Lays out the tree under `root` within `width` × `height` logical pixels.
    fn compute(&mut self, root: NodeId, width: f32, height: f32);
    fn layout(&self, id: NodeId) -> Option<NodeLayout>;
}

/// [`LayoutEngine`] backed by Taffy. Taffy caches results internally, so only
/// subtrees whose styles or children changed are recomputed.
#[derive(Default)]
pub struct TaffyLayout {
    tree: TaffyTree<()>,
    ids: HashMap<NodeId, taffy::NodeId>,
}

impl TaffyLayout {
    pub fn new() -> Self {
        Self::default()
    }

    fn taffy_id(&self, id: NodeId) -> Option<taffy::NodeId> {
        self.ids.get(&id).copied()
    }
}

impl LayoutEngine for TaffyLayout {
    fn create_node(&mut self, id: NodeId, style: &Style) {
        let node = self
            .tree
            .new_leaf(style.clone())
            .expect("taffy new_leaf is infallible");
        if let Some(old) = self.ids.insert(id, node) {
            let _ = self.tree.remove(old);
        }
    }

    fn remove_node(&mut self, id: NodeId) {
        if let Some(node) = self.ids.remove(&id) {
            let _ = self.tree.remove(node);
        }
    }

    fn set_style(&mut self, id: NodeId, style: &Style) {
        if let Some(node) = self.taffy_id(id) {
            let _ = self.tree.set_style(node, style.clone());
        }
    }

    fn set_children(&mut self, parent: NodeId, children: &[NodeId]) {
        let Some(parent) = self.taffy_id(parent) else {
            return;
        };
        let children: Vec<_> = children.iter().filter_map(|c| self.taffy_id(*c)).collect();
        let _ = self.tree.set_children(parent, &children);
    }

    fn compute(&mut self, root: NodeId, width: f32, height: f32) {
        if let Some(root) = self.taffy_id(root) {
            let available = Size {
                width: AvailableSpace::Definite(width),
                height: AvailableSpace::Definite(height),
            };
            if let Err(e) = self.tree.compute_layout(root, available) {
                tracing::error!("layout failed: {e}");
            }
        }
    }

    fn layout(&self, id: NodeId) -> Option<NodeLayout> {
        let l = self.tree.layout(self.taffy_id(id)?).ok()?;
        Some(NodeLayout {
            x: l.location.x,
            y: l.location.y,
            width: l.size.width,
            height: l.size.height,
            border: [l.border.top, l.border.right, l.border.bottom, l.border.left],
        })
    }
}
