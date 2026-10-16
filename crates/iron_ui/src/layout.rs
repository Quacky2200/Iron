use rustc_hash::FxHashMap as HashMap;

use iron_core::NodeId;
use taffy::{AvailableSpace, Size, Style, TaffyTree, compute_leaf_layout};

use crate::text::AvailableWidth;

/// Sizes a measured leaf (text): given a known content width, if any, and the
/// available width, returns its content size (width, height).
pub type Measure<'a> = dyn FnMut(NodeId, Option<f32>, AvailableWidth) -> (f32, f32) + 'a;

/// A node's computed layout, relative to its parent's border box.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct NodeLayout {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    /// Resolved border widths: top, right, bottom, left.
    pub border: [f32; 4],
    /// Resolved padding: top, right, bottom, left.
    pub padding: [f32; 4],
    /// How far the content can scroll (x, y): its reachable extent beyond the padding box.
    pub scroll_max: [f32; 2],
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
    /// Marks a leaf whose size comes from `compute`'s `measure` callback (text).
    fn set_measured(&mut self, id: NodeId, measured: bool);
    /// Marks a node for relayout without a style change (e.g. its text changed).
    fn mark_dirty(&mut self, id: NodeId);
    /// Lays out the tree under `root` within `width` × `height` logical pixels,
    /// calling `measure` for measured leaves.
    fn compute(&mut self, root: NodeId, width: f32, height: f32, measure: &mut Measure<'_>);
    fn layout(&self, id: NodeId) -> Option<NodeLayout>;
}

/// [`LayoutEngine`] backed by Taffy. Taffy caches results internally, so only
/// subtrees whose styles or children changed are recomputed.
pub struct TaffyLayout {
    /// Node context: the Iron id of measured leaves.
    tree: TaffyTree<NodeId>,
    ids: HashMap<NodeId, taffy::NodeId>,
}

impl TaffyLayout {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            ids: HashMap::default(),
        }
    }

    fn taffy_id(&self, id: NodeId) -> Option<taffy::NodeId> {
        self.ids.get(&id).copied()
    }
}

impl Default for TaffyLayout {
    fn default() -> Self {
        Self::new()
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

    fn set_measured(&mut self, id: NodeId, measured: bool) {
        if let Some(node) = self.taffy_id(id) {
            let _ = self.tree.set_node_context(node, measured.then_some(id));
        }
    }

    fn mark_dirty(&mut self, id: NodeId) {
        if let Some(node) = self.taffy_id(id) {
            let _ = self.tree.mark_dirty(node);
        }
    }

    fn compute(&mut self, root: NodeId, width: f32, height: f32, measure: &mut Measure<'_>) {
        let Some(root) = self.taffy_id(root) else {
            return;
        };
        let available = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        let result =
            self.tree
                .compute_layout_with_measure(root, available, |inputs, _, context, style| {
                    // Taffy handles padding and borders; the inner closure sizes content.
                    compute_leaf_layout(
                        inputs,
                        style,
                        |_, _| 0.0,
                        |known, available| {
                            let Some(id) = context.as_deref().copied() else {
                                return Size::ZERO;
                            };
                            let width = match available.width {
                                AvailableSpace::Definite(w) => AvailableWidth::Definite(w),
                                AvailableSpace::MinContent => AvailableWidth::MinContent,
                                AvailableSpace::MaxContent => AvailableWidth::MaxContent,
                            };
                            let (w, h) = measure(id, known.width, width);
                            Size {
                                width: known.width.unwrap_or(w),
                                height: known.height.unwrap_or(h),
                            }
                        },
                    )
                });
        if let Err(e) = result {
            tracing::error!("layout failed: {e}");
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
            padding: [
                l.padding.top,
                l.padding.right,
                l.padding.bottom,
                l.padding.left,
            ],
            scroll_max: [l.scroll_width(), l.scroll_height()],
        })
    }
}
