use rustc_hash::FxHashMap as HashMap;

use iron_core::NodeId;
use taffy::detailed_info::DetailedGridInfo;
use taffy::{
    AvailableSpace, BlockContext, Cache, CacheTree, ClearState, DetailedLayoutInfo, Display,
    Layout, LayoutBlockContainer, LayoutContainingBlock, LayoutFlexboxContainer,
    LayoutGridContainer, LayoutInput, LayoutOutput, LayoutPartialTree, RoundTree, RunMode, Size,
    Style, TraversePartialTree, TraverseTree, compute_block_layout, compute_cached_layout,
    compute_flexbox_layout, compute_grid_layout, compute_hidden_layout, compute_leaf_layout,
    compute_oof_layout, compute_root_layout, round_layout,
};

use crate::text::AvailableWidth;

/// Sizes a measured leaf (text, fields, images): given the known content width and
/// height, if any, and the available width, returns its content size (width, height).
pub type Measure<'a> =
    dyn FnMut(NodeId, (Option<f32>, Option<f32>), AvailableWidth) -> (f32, f32) + 'a;

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

/// [`LayoutEngine`] backed by Taffy's algorithms over Iron's own node storage.
///
/// Taffy's ready-made `TaffyTree` can't evaluate `calc()` (its `resolve_calc_value`
/// returns 0), so this implements Taffy's low-level tree traits itself, mirroring
/// `TaffyTree`, and resolves calc handles through [`crate::calc`]. Layout results
/// are cached per node, so only subtrees whose styles or children changed are
/// recomputed.
pub struct TaffyLayout {
    nodes: HashMap<u64, NodeData>,
}

struct NodeData {
    style: Style,
    children: Vec<taffy::NodeId>,
    parent: Option<taffy::NodeId>,
    /// A leaf sized by the `measure` callback (text, text fields).
    measured: bool,
    unrounded: Layout,
    final_layout: Layout,
    /// Out-of-flow boxes whose containing block is this node.
    hoisted: Vec<taffy::NodeId>,
    cache: Cache,
    detailed: DetailedLayoutInfo,
}

impl NodeData {
    fn new(style: Style) -> Self {
        Self {
            style,
            children: Vec::new(),
            parent: None,
            measured: false,
            unrounded: Layout::new(),
            final_layout: Layout::new(),
            hoisted: Vec::new(),
            cache: Cache::new(),
            detailed: DetailedLayoutInfo::None,
        }
    }
}

fn key(id: NodeId) -> taffy::NodeId {
    taffy::NodeId::from(u64::from(id.0))
}

impl TaffyLayout {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::default(),
        }
    }

    fn node(&self, id: taffy::NodeId) -> &NodeData {
        &self.nodes[&u64::from(id)]
    }

    fn node_mut(&mut self, id: taffy::NodeId) -> &mut NodeData {
        self.nodes.get_mut(&u64::from(id)).expect("node exists")
    }

    /// Clears the cached layout of `id` and its ancestors (stopping at one that was
    /// already clear, as `TaffyTree` does).
    fn mark_dirty_up(&mut self, id: taffy::NodeId) {
        let mut at = Some(id);
        while let Some(n) = at {
            let Some(node) = self.nodes.get_mut(&u64::from(n)) else {
                return;
            };
            if matches!(node.cache.clear(), ClearState::AlreadyEmpty) {
                return;
            }
            at = node.parent;
        }
    }
}

impl Default for TaffyLayout {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutEngine for TaffyLayout {
    fn create_node(&mut self, id: NodeId, style: &Style) {
        self.remove_node(id);
        self.nodes
            .insert(u64::from(id.0), NodeData::new(style.clone()));
    }

    fn remove_node(&mut self, id: NodeId) {
        let Some(node) = self.nodes.remove(&u64::from(id.0)) else {
            return;
        };
        let k = key(id);
        if let Some(parent) = node.parent
            && let Some(p) = self.nodes.get_mut(&u64::from(parent))
        {
            p.children.retain(|c| *c != k);
            self.mark_dirty_up(parent);
        }
        for child in node.children {
            if let Some(c) = self.nodes.get_mut(&u64::from(child))
                && c.parent == Some(k)
            {
                c.parent = None;
            }
        }
    }

    fn set_style(&mut self, id: NodeId, style: &Style) {
        let k = key(id);
        if let Some(node) = self.nodes.get_mut(&u64::from(k)) {
            node.style = style.clone();
            self.mark_dirty_up(k);
        }
    }

    fn set_children(&mut self, parent: NodeId, children: &[NodeId]) {
        let pk = key(parent);
        if !self.nodes.contains_key(&u64::from(pk)) {
            return;
        }
        let new: Vec<taffy::NodeId> = children
            .iter()
            .map(|c| key(*c))
            .filter(|c| self.nodes.contains_key(&u64::from(*c)))
            .collect();
        let old = std::mem::replace(&mut self.node_mut(pk).children, new.clone());
        for c in old {
            if let Some(n) = self.nodes.get_mut(&u64::from(c))
                && n.parent == Some(pk)
            {
                n.parent = None;
            }
        }
        for c in new {
            // A child moved from another parent leaves that parent's list too.
            let previous = self.node(c).parent;
            if let Some(prev) = previous.filter(|p| *p != pk)
                && let Some(p) = self.nodes.get_mut(&u64::from(prev))
            {
                p.children.retain(|x| *x != c);
                self.mark_dirty_up(prev);
            }
            self.node_mut(c).parent = Some(pk);
        }
        self.mark_dirty_up(pk);
    }

    fn set_measured(&mut self, id: NodeId, measured: bool) {
        let k = key(id);
        if let Some(node) = self.nodes.get_mut(&u64::from(k))
            && node.measured != measured
        {
            node.measured = measured;
            self.mark_dirty_up(k);
        }
    }

    fn mark_dirty(&mut self, id: NodeId) {
        self.mark_dirty_up(key(id));
    }

    fn compute(&mut self, root: NodeId, width: f32, height: f32, measure: &mut Measure<'_>) {
        let root = key(root);
        if !self.nodes.contains_key(&u64::from(root)) {
            return;
        }
        let available = Size {
            width: AvailableSpace::Definite(width),
            height: AvailableSpace::Definite(height),
        };
        let mut view = View {
            layout: self,
            measure,
        };
        compute_root_layout(&mut view, root, available);
        round_layout(&mut view, root);
    }

    fn layout(&self, id: NodeId) -> Option<NodeLayout> {
        let l = &self.nodes.get(&u64::from(id.0))?.final_layout;
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

/// A layout pass over [`TaffyLayout`]: the tree plus the measure callback for leaves.
/// Implements Taffy's tree traits like `TaffyTree`'s own view does.
struct View<'a, 'm> {
    layout: &'a mut TaffyLayout,
    measure: &'a mut Measure<'m>,
}

impl View<'_, '_> {
    fn style(&self, id: taffy::NodeId) -> &Style {
        &self.layout.node(id).style
    }

    fn compute_child(
        &mut self,
        node_id: taffy::NodeId,
        inputs: LayoutInput,
        block_ctx: Option<&mut BlockContext<'_>>,
    ) -> LayoutOutput {
        if inputs.run_mode == RunMode::PerformHiddenLayout {
            return compute_hidden_layout(self, node_id);
        }
        compute_cached_layout(self, node_id, inputs, |tree, node_id, inputs| {
            let display = tree.style(node_id).display;
            let has_children = tree.child_count(node_id) > 0;
            let mut output = match (display, has_children) {
                (Display::None, _) => compute_hidden_layout(tree, node_id),
                (Display::Block, true) => compute_block_layout(tree, node_id, inputs, block_ctx),
                (Display::FlowRoot, true) => compute_block_layout(tree, node_id, inputs, None),
                (Display::Flex, true) => compute_flexbox_layout(tree, node_id, inputs),
                (Display::Grid, true) => compute_grid_layout(tree, node_id, inputs),
                (_, false) => tree.measure_leaf(node_id, inputs),
            };
            if inputs.run_mode == RunMode::PerformLayout {
                compute_oof_layout(tree, node_id, &mut output);
            }
            output
        })
    }

    /// A leaf: Taffy handles padding and borders; `measure` sizes the content of
    /// measured leaves (text), others have none.
    fn measure_leaf(&mut self, node_id: taffy::NodeId, inputs: LayoutInput) -> LayoutOutput {
        let node = self.layout.node(node_id);
        let measured = node.measured.then_some(NodeId(u64::from(node_id) as u32));
        let measure = &mut *self.measure;
        compute_leaf_layout(
            inputs,
            &node.style,
            crate::calc::resolve,
            |known, available| {
                let Some(id) = measured else {
                    return Size::ZERO;
                };
                let width = match available.width {
                    AvailableSpace::Definite(w) => AvailableWidth::Definite(w),
                    AvailableSpace::MinContent => AvailableWidth::MinContent,
                    AvailableSpace::MaxContent => AvailableWidth::MaxContent,
                };
                let (w, h) = measure(id, (known.width, known.height), width);
                Size {
                    width: known.width.unwrap_or(w),
                    height: known.height.unwrap_or(h),
                }
            },
        )
    }
}

impl TraversePartialTree for View<'_, '_> {
    type ChildIter<'a>
        = std::iter::Copied<std::slice::Iter<'a, taffy::NodeId>>
    where
        Self: 'a;

    fn child_ids(&self, parent: taffy::NodeId) -> Self::ChildIter<'_> {
        self.layout.node(parent).children.iter().copied()
    }

    fn child_count(&self, parent: taffy::NodeId) -> usize {
        self.layout.node(parent).children.len()
    }

    fn get_child_id(&self, parent: taffy::NodeId, index: usize) -> taffy::NodeId {
        self.layout.node(parent).children[index]
    }
}

impl TraverseTree for View<'_, '_> {}

impl LayoutPartialTree for View<'_, '_> {
    type CoreContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type CustomIdent = String;

    fn get_core_container_style(&self, node_id: taffy::NodeId) -> Self::CoreContainerStyle<'_> {
        self.style(node_id)
    }

    fn set_unrounded_layout(&mut self, node_id: taffy::NodeId, layout: &Layout) {
        self.layout.node_mut(node_id).unrounded = *layout;
    }

    fn resolve_calc_value(&self, val: *const (), basis: f32) -> f32 {
        crate::calc::resolve(val, basis)
    }

    fn compute_child_layout(
        &mut self,
        node_id: taffy::NodeId,
        inputs: LayoutInput,
    ) -> LayoutOutput {
        self.compute_child(node_id, inputs, None)
    }
}

impl LayoutContainingBlock for View<'_, '_> {
    type OofItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_oof_item_style(&self, node_id: taffy::NodeId) -> Self::OofItemStyle<'_> {
        self.style(node_id)
    }

    fn clear_hoisted_children(&mut self, node_id: taffy::NodeId) {
        self.layout.node_mut(node_id).hoisted.clear();
    }

    fn add_hoisted_children(&mut self, node_id: taffy::NodeId, hoisted: &[taffy::NodeId]) {
        self.layout
            .node_mut(node_id)
            .hoisted
            .extend_from_slice(hoisted);
    }

    fn get_detailed_layout_info(&self, node_id: taffy::NodeId) -> &DetailedLayoutInfo {
        &self.layout.node(node_id).detailed
    }
}

impl CacheTree for View<'_, '_> {
    fn cache_get(&mut self, node_id: taffy::NodeId, input: &LayoutInput) -> Option<LayoutOutput> {
        self.layout.node_mut(node_id).cache.get(input)
    }

    fn cache_store(&mut self, node_id: taffy::NodeId, input: &LayoutInput, output: LayoutOutput) {
        self.layout.node_mut(node_id).cache.store(input, output)
    }

    fn cache_clear(&mut self, node_id: taffy::NodeId) {
        self.layout.node_mut(node_id).cache.clear();
    }
}

impl LayoutBlockContainer for View<'_, '_> {
    type BlockContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type BlockItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_block_container_style(&self, node_id: taffy::NodeId) -> Self::BlockContainerStyle<'_> {
        self.style(node_id)
    }

    fn get_block_child_style(&self, child: taffy::NodeId) -> Self::BlockItemStyle<'_> {
        self.style(child)
    }

    fn compute_block_child_layout(
        &mut self,
        node_id: taffy::NodeId,
        inputs: LayoutInput,
        block_ctx: Option<&mut BlockContext<'_>>,
    ) -> LayoutOutput {
        self.compute_child(node_id, inputs, block_ctx)
    }
}

impl LayoutFlexboxContainer for View<'_, '_> {
    type FlexboxContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type FlexboxItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_flexbox_container_style(
        &self,
        node_id: taffy::NodeId,
    ) -> Self::FlexboxContainerStyle<'_> {
        self.style(node_id)
    }

    fn get_flexbox_child_style(&self, child: taffy::NodeId) -> Self::FlexboxItemStyle<'_> {
        self.style(child)
    }
}

impl LayoutGridContainer for View<'_, '_> {
    type GridContainerStyle<'a>
        = &'a Style
    where
        Self: 'a;
    type GridItemStyle<'a>
        = &'a Style
    where
        Self: 'a;

    fn get_grid_container_style(&self, node_id: taffy::NodeId) -> Self::GridContainerStyle<'_> {
        self.style(node_id)
    }

    fn get_grid_child_style(&self, child: taffy::NodeId) -> Self::GridItemStyle<'_> {
        self.style(child)
    }

    fn set_detailed_grid_info(&mut self, node_id: taffy::NodeId, info: DetailedGridInfo) {
        self.layout.node_mut(node_id).detailed = DetailedLayoutInfo::Grid(Box::new(info));
    }
}

impl RoundTree for View<'_, '_> {
    fn get_unrounded_layout(&self, node_id: taffy::NodeId) -> Layout {
        self.layout.node(node_id).unrounded
    }

    fn set_final_layout(&mut self, node_id: taffy::NodeId, layout: &Layout) {
        self.layout.node_mut(node_id).final_layout = *layout;
    }

    fn is_out_of_flow(&self, node_id: taffy::NodeId) -> bool {
        let style = self.style(node_id);
        style.position.is_out_of_flow() && style.display != Display::None
    }

    fn hoisted_child_count(&self, node_id: taffy::NodeId) -> usize {
        self.layout.node(node_id).hoisted.len()
    }

    fn get_hoisted_child_id(&self, node_id: taffy::NodeId, index: usize) -> taffy::NodeId {
        self.layout.node(node_id).hoisted[index]
    }
}
