use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::fmt;

use iron_core::{Commit, NodeId, Value};
use taffy::{Dimension, Display, Size, Style};

use crate::draw::{DrawCmd, ImageFill, Quad, Rect, ShadowShape};
use crate::layout::{LayoutEngine, TaffyLayout};
use crate::opcodes;
use crate::paint::{Shadow, resolve_radii};
use crate::style::{Visual, apply_style};

/// The root container every React root renders into.
pub const ROOT: NodeId = NodeId(0);

/// A problem with one op. The op is skipped; the tree stays valid.
#[derive(Clone, Debug, PartialEq)]
pub struct UiError(pub String);

impl fmt::Display for UiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for UiError {}

struct Node {
    kind: String,
    parent: Option<NodeId>,
    children: Vec<NodeId>,
    style: Style,
    visual: Visual,
    /// The characters of a `#text` node.
    text: Option<String>,
}

/// Element type of text nodes created with `CREATE_TEXT`.
pub const TEXT_NODE: &str = "#text";

/// Element types whose children are inline content laid out by the text engine.
fn is_inline_container(kind: &str) -> bool {
    matches!(kind, "text" | "span")
}

/// Element types that are inline content when inside a text container.
fn is_inline(kind: &str) -> bool {
    matches!(kind, "span" | TEXT_NODE)
}

/// The UI node tree, owned by Rust and built from React commits (DESIGN.md §3, §5).
pub struct UiTree<L: LayoutEngine = TaffyLayout> {
    /// Boxed: nodes are large (~700 bytes), so the map stays cheap to grow.
    nodes: HashMap<NodeId, Box<Node>>,
    layout: L,
    size: (f32, f32),
    needs_layout: bool,
    /// Absolute rects from the last layout, for drawing and hit testing.
    rects: HashMap<NodeId, Rect>,
    /// Text blocks whose content or structure changed since they were last laid out.
    dirty_text: HashSet<NodeId>,
}

impl UiTree<TaffyLayout> {
    pub fn new(width: f32, height: f32) -> Self {
        Self::with_layout(TaffyLayout::new(), width, height)
    }
}

impl<L: LayoutEngine> UiTree<L> {
    pub fn with_layout(mut layout: L, width: f32, height: f32) -> Self {
        let style = root_style(width, height);
        layout.create_node(ROOT, &style);
        let mut nodes = HashMap::default();
        nodes.insert(
            ROOT,
            Box::new(Node {
                kind: "root".into(),
                parent: None,
                children: Vec::new(),
                style,
                visual: Visual::default(),
                text: None,
            }),
        );
        Self {
            nodes,
            layout,
            size: (width, height),
            needs_layout: true,
            rects: HashMap::default(),
            dirty_text: HashSet::default(),
        }
    }

    /// Sets the root size in logical pixels (the window's inner size / scale factor).
    pub fn set_size(&mut self, width: f32, height: f32) {
        if self.size == (width, height) {
            return;
        }
        self.size = (width, height);
        let style = root_style(width, height);
        self.layout.set_style(ROOT, &style);
        self.nodes.get_mut(&ROOT).expect("root exists").style = style;
        self.needs_layout = true;
    }

    pub fn size(&self) -> (f32, f32) {
        self.size
    }

    pub fn len(&self) -> usize {
        self.nodes.len() - 1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn contains(&self, id: NodeId) -> bool {
        self.nodes.contains_key(&id)
    }

    pub fn kind(&self, id: NodeId) -> Option<&str> {
        self.nodes.get(&id).map(|n| n.kind.as_str())
    }

    pub fn parent(&self, id: NodeId) -> Option<NodeId> {
        self.nodes.get(&id)?.parent
    }

    /// The characters of a `#text` node.
    pub fn text(&self, id: NodeId) -> Option<&str> {
        self.nodes.get(&id)?.text.as_deref()
    }

    /// Whether `id` is laid out as a block of text: a `text` element, or a span or
    /// text node outside any text container (an anonymous text block).
    pub fn is_text_root(&self, id: NodeId) -> bool {
        let Some(node) = self.nodes.get(&id) else {
            return false;
        };
        if node.kind == "text" {
            return true;
        }
        is_inline(&node.kind) && !self.inside_inline_container(id)
    }

    fn inside_inline_container(&self, id: NodeId) -> bool {
        self.parent(id)
            .and_then(|p| self.nodes.get(&p))
            .is_some_and(|p| is_inline_container(&p.kind))
    }

    /// The text block that lays out `id`, if `id` is text content or a text block.
    pub fn text_root_of(&self, id: NodeId) -> Option<NodeId> {
        let mut cursor = id;
        loop {
            if self.is_text_root(cursor) {
                return Some(cursor);
            }
            let node = self.nodes.get(&cursor)?;
            if !is_inline(&node.kind) && !is_inline_container(&node.kind) {
                return None;
            }
            cursor = node.parent?;
        }
    }

    /// The plain text of a text block: all `#text` descendants in order.
    pub fn text_content(&self, id: NodeId) -> String {
        let mut out = String::new();
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            let Some(node) = self.nodes.get(&n) else {
                continue;
            };
            if let Some(t) = &node.text {
                out.push_str(t);
            }
            stack.extend(node.children.iter().rev());
        }
        out
    }

    /// Text blocks changed since the last call (clears the set).
    pub fn take_dirty_text(&mut self) -> Vec<NodeId> {
        let dirty: Vec<NodeId> = self.dirty_text.drain().collect();
        dirty
            .into_iter()
            .filter(|id| self.nodes.contains_key(id))
            .collect()
    }

    pub fn children(&self, id: NodeId) -> &[NodeId] {
        self.nodes.get(&id).map_or(&[], |n| &n.children)
    }

    /// Applies one commit. Bad ops are skipped and returned; the rest still apply.
    pub fn apply_commit(&mut self, commit: &Commit) -> Vec<UiError> {
        let mut errors = Vec::new();
        let mut children_changed = HashSet::default();
        let mut styles_changed = HashSet::default();
        let mut text_changed = HashSet::default();

        for op in &commit.ops {
            if let Err(e) = self.apply_op(
                op,
                &mut children_changed,
                &mut styles_changed,
                &mut text_changed,
            ) {
                errors.push(e);
            }
        }

        // Sync with the layout engine once per commit, not once per op.
        for id in styles_changed {
            if let Some(node) = self.nodes.get(&id) {
                self.layout.set_style(id, &node.style);
            }
        }
        for id in &children_changed {
            if let Some(node) = self.nodes.get(id) {
                if is_inline_container(&node.kind) {
                    // Spans and text nodes inside are laid out by the text engine.
                    self.layout.set_children(*id, &[]);
                } else {
                    self.layout.set_children(*id, &node.children);
                }
            }
        }
        // Text blocks whose content changed. A changed parent may also have new
        // text children, which are text blocks of their own.
        let touched = children_changed
            .iter()
            .flat_map(|id| {
                let kids = self.children(*id).to_vec();
                std::iter::once(*id).chain(kids)
            })
            .chain(text_changed.iter().copied())
            .collect::<Vec<_>>();
        for id in touched {
            if let Some(root) = self.text_root_of(id) {
                self.dirty_text.insert(root);
            }
        }
        if !commit.ops.is_empty() {
            self.needs_layout = true;
        }
        errors
    }

    fn apply_op(
        &mut self,
        op: &Value,
        children_changed: &mut HashSet<NodeId>,
        styles_changed: &mut HashSet<NodeId>,
        text_changed: &mut HashSet<NodeId>,
    ) -> Result<(), UiError> {
        let Value::Array(args) = op else {
            return Err(UiError(format!("op must be an array, got {op:?}")));
        };
        let opcode = args
            .first()
            .and_then(Value::as_f64)
            .ok_or_else(|| UiError(format!("op is missing its opcode: {op:?}")))?;
        let arg = |i: usize| {
            args.get(i)
                .ok_or_else(|| UiError(format!("op is missing argument {i}: {op:?}")))
        };
        let id = |i: usize| node_id(arg(i)?);

        match opcode as u32 {
            opcodes::CREATE => {
                let id = id(1)?;
                if id == ROOT || self.nodes.contains_key(&id) {
                    return Err(UiError(format!("node {id} already exists")));
                }
                let kind = arg(2)?
                    .as_str()
                    .ok_or_else(|| UiError(format!("CREATE needs an element type: {op:?}")))?;
                let style = Style::default();
                self.layout.create_node(id, &style);
                self.nodes.insert(
                    id,
                    Box::new(Node {
                        kind: kind.to_owned(),
                        parent: None,
                        children: Vec::new(),
                        style,
                        visual: Visual::default(),
                        text: None,
                    }),
                );
            }
            opcodes::CREATE_TEXT => {
                let id = id(1)?;
                if id == ROOT || self.nodes.contains_key(&id) {
                    return Err(UiError(format!("node {id} already exists")));
                }
                let text = arg(2)?
                    .as_str()
                    .ok_or_else(|| UiError(format!("CREATE_TEXT needs a string: {op:?}")))?;
                let style = Style::default();
                self.layout.create_node(id, &style);
                self.nodes.insert(
                    id,
                    Box::new(Node {
                        kind: TEXT_NODE.to_owned(),
                        parent: None,
                        children: Vec::new(),
                        style,
                        visual: Visual::default(),
                        text: Some(text.to_owned()),
                    }),
                );
            }
            opcodes::SET_TEXT => {
                let id = id(1)?;
                let new_text = arg(2)?
                    .as_str()
                    .ok_or_else(|| UiError(format!("SET_TEXT needs a string: {op:?}")))?
                    .to_owned();
                let node = self.node_mut(id)?;
                if node.kind != TEXT_NODE {
                    return Err(UiError(format!(
                        "node {id} is a <{}>, not a text node",
                        node.kind
                    )));
                }
                node.text = Some(new_text);
                // The text block containing this node must be measured again.
                text_changed.insert(id);
            }
            opcodes::SET_STYLE => {
                let id = id(1)?;
                let node = self.node_mut(id)?;
                let pairs = &args[2..];
                if pairs.len() % 2 != 0 {
                    return Err(UiError(format!("SET_STYLE needs prop/value pairs: {op:?}")));
                }
                let mut problems = Vec::new();
                for pair in pairs.chunks(2) {
                    let Some(prop) = pair[0].as_str() else {
                        problems.push(format!(
                            "style property name must be a string: {:?}",
                            pair[0]
                        ));
                        continue;
                    };
                    if let Err(e) = apply_style(&mut node.style, &mut node.visual, prop, &pair[1]) {
                        problems.push(e.to_string());
                    }
                }
                styles_changed.insert(id);
                if !problems.is_empty() {
                    return Err(UiError(format!("node {id}: {}", problems.join("; "))));
                }
            }
            opcodes::APPEND_CHILD => {
                let (parent, child) = (id(1)?, id(2)?);
                self.attach(parent, child, None, children_changed)?;
            }
            opcodes::INSERT_BEFORE => {
                let (parent, child, before) = (id(1)?, id(2)?, id(3)?);
                self.attach(parent, child, Some(before), children_changed)?;
            }
            opcodes::REMOVE_CHILD => {
                let (parent, child) = (id(1)?, id(2)?);
                if self.parent(child) != Some(parent) {
                    return Err(UiError(format!("node {child} is not a child of {parent}")));
                }
                self.detach(child, children_changed);
                self.destroy_subtree(child);
            }
            other => return Err(UiError(format!("unknown opcode {other}"))),
        }
        Ok(())
    }

    fn node_mut(&mut self, id: NodeId) -> Result<&mut Node, UiError> {
        self.nodes
            .get_mut(&id)
            .map(|n| &mut **n)
            .ok_or_else(|| UiError(format!("unknown node {id}")))
    }

    fn attach(
        &mut self,
        parent: NodeId,
        child: NodeId,
        before: Option<NodeId>,
        children_changed: &mut HashSet<NodeId>,
    ) -> Result<(), UiError> {
        match self.nodes.get(&parent) {
            None => return Err(UiError(format!("unknown parent {parent}"))),
            Some(p) if p.kind == TEXT_NODE => {
                return Err(UiError(format!("text node {parent} can't have children")));
            }
            Some(_) => {}
        }
        if child == ROOT || !self.nodes.contains_key(&child) {
            return Err(UiError(format!("unknown child {child}")));
        }
        if before == Some(child) {
            return Err(UiError(format!("cannot insert node {child} before itself")));
        }
        // Refuse cycles: `child` must not be `parent` or one of its ancestors.
        let mut cursor = Some(parent);
        while let Some(n) = cursor {
            if n == child {
                return Err(UiError(format!("cannot insert node {child} inside itself")));
            }
            cursor = self.parent(n);
        }
        if let Some(before) = before
            && self.parent(before) != Some(parent)
        {
            return Err(UiError(format!("node {before} is not a child of {parent}")));
        }

        // Moving an attached node: take it out of its old place first.
        self.detach(child, children_changed);

        let siblings = &mut self.nodes.get_mut(&parent).expect("checked").children;
        let index = match before {
            Some(b) => siblings.iter().position(|c| *c == b).expect("checked"),
            None => siblings.len(),
        };
        siblings.insert(index, child);
        self.nodes.get_mut(&child).expect("checked").parent = Some(parent);
        children_changed.insert(parent);
        Ok(())
    }

    fn detach(&mut self, child: NodeId, children_changed: &mut HashSet<NodeId>) {
        let Some(old_parent) = self.nodes.get_mut(&child).and_then(|n| n.parent.take()) else {
            return;
        };
        if let Some(p) = self.nodes.get_mut(&old_parent) {
            p.children.retain(|c| *c != child);
            children_changed.insert(old_parent);
        }
    }

    fn destroy_subtree(&mut self, id: NodeId) {
        let mut stack = vec![id];
        while let Some(id) = stack.pop() {
            if let Some(node) = self.nodes.remove(&id) {
                stack.extend(node.children);
                self.layout.remove_node(id);
                self.rects.remove(&id);
            }
        }
    }

    /// Runs layout if anything changed since the last run. Returns whether it ran.
    pub fn layout_if_needed(&mut self) -> bool {
        if !self.needs_layout {
            return false;
        }
        self.layout.compute(ROOT, self.size.0, self.size.1);
        self.rects.clear();
        let mut stack = vec![(ROOT, 0.0_f32, 0.0_f32)];
        while let Some((id, parent_x, parent_y)) = stack.pop() {
            let Some(l) = self.layout.layout(id) else {
                continue;
            };
            let (x, y) = (parent_x + l.x, parent_y + l.y);
            self.rects.insert(id, Rect::new(x, y, l.width, l.height));
            if let Some(node) = self.nodes.get(&id)
                && !is_inline_container(&node.kind)
            {
                // Inline content inside text is positioned by the text engine.
                stack.extend(node.children.iter().map(|c| (*c, x, y)));
            }
        }
        self.needs_layout = false;
        true
    }

    /// The node's absolute rect from the most recent layout.
    pub fn rect(&self, id: NodeId) -> Option<Rect> {
        self.rects.get(&id).copied()
    }

    /// Lays out if needed, then lists the paint commands for the whole tree.
    ///
    /// Per node, in order (matching CSS painting): `BackdropBlur` (if any), then
    /// `PushLayer` when the node needs one (a `filter` blur, or group opacity over
    /// more than one command), then outer `BoxShadow`s, the `Quad`, `InsetShadow`s,
    /// the children, and `PopLayer`. `display: none` and fully transparent subtrees
    /// are skipped. A node with opacity but nothing to group just scales its alpha.
    pub fn draw_list(&mut self) -> Vec<DrawCmd> {
        self.layout_if_needed();
        let mut out = Vec::new();
        self.collect(ROOT, &mut out);
        out
    }

    fn collect(&self, id: NodeId, out: &mut Vec<DrawCmd>) {
        let Some(node) = self.nodes.get(&id) else {
            return;
        };
        let v = &node.visual;
        if node.style.display == Display::None || v.opacity <= 0.0 {
            return;
        }
        let (Some(&rect), Some(l)) = (self.rects.get(&id), self.layout.layout(id)) else {
            return;
        };
        let radii = resolve_radii(&v.border_radius, rect.width, rect.height);

        let mut own = Vec::new();
        let shadow = |s: &Shadow| ShadowShape {
            rect,
            radii,
            offset_x: s.offset_x,
            offset_y: s.offset_y,
            blur: s.blur,
            spread: s.spread,
            color: s.color,
        };
        // CSS paints the first shadow on top, so draw the list back to front.
        for s in v
            .shadows
            .iter()
            .rev()
            .filter(|s| !s.inset && s.color.a > 0.0)
        {
            own.push(DrawCmd::BoxShadow(shadow(s)));
        }
        let quad = Quad {
            rect,
            background: v.background,
            image: v.background_image.as_ref().map(|image| ImageFill {
                image: image.clone(),
                size: v.background_size,
                position: v.background_position,
                repeat: v.background_repeat,
            }),
            border_color: v.border_color,
            border: l.border,
            radii,
        };
        if quad.is_visible() {
            own.push(DrawCmd::Quad(quad));
        }
        for s in v
            .shadows
            .iter()
            .rev()
            .filter(|s| s.inset && s.color.a > 0.0)
        {
            own.push(DrawCmd::InsetShadow {
                shadow: shadow(s),
                border: l.border,
            });
        }

        if v.backdrop_blur > 0.0 {
            out.push(DrawCmd::BackdropBlur {
                rect,
                radii,
                blur: v.backdrop_blur,
            });
        }

        let group_opacity = v.opacity < 1.0 && (!node.children.is_empty() || own.len() > 1);
        if v.filter_blur > 0.0 || group_opacity {
            let start = out.len();
            out.push(DrawCmd::PushLayer {
                bounds: rect,
                blur: v.filter_blur,
                opacity: v.opacity,
            });
            out.extend(own);
            for child in &node.children {
                self.collect(*child, out);
            }
            // The layer must cover everything drawn into it, plus the blur's spread.
            let content = out[start + 1..]
                .iter()
                .filter_map(DrawCmd::bounds)
                .fold(rect, Rect::union);
            if let DrawCmd::PushLayer { bounds, .. } = &mut out[start] {
                *bounds = content.expand(3.0 * v.filter_blur);
            }
            out.push(DrawCmd::PopLayer);
        } else {
            if v.opacity < 1.0 {
                for cmd in &mut own {
                    cmd.scale_alpha(v.opacity);
                }
            }
            out.extend(own);
            for child in &node.children {
                self.collect(*child, out);
            }
        }
    }
}

fn root_style(width: f32, height: f32) -> Style {
    Style {
        size: Size {
            width: Dimension::length(width),
            height: Dimension::length(height),
        },
        ..Style::default()
    }
}

fn node_id(v: &Value) -> Result<NodeId, UiError> {
    match v.as_f64() {
        Some(n) if n >= 0.0 && n.fract() == 0.0 && n <= f64::from(u32::MAX) => Ok(NodeId(n as u32)),
        _ => Err(UiError(format!("invalid node id {v:?}"))),
    }
}
