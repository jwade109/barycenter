#![allow(dead_code)]
#![allow(unused)]

use crate::svg::write_svg;
use bary_core::prelude::*;
use log::*;

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodePath(String);

impl NodePath {
    pub fn new() -> Self {
        Self("/".into())
    }

    pub fn append(&self, s: impl Into<String>) -> Self {
        let mut ret = self.clone();
        if ret.0.chars().last() != Some('/') {
            ret.0.push('/');
        }
        ret.0.extend(s.into().chars());
        ret
    }
}

impl std::fmt::Debug for NodePath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone)]
pub enum IdSalt {
    Integer(i64),
    String(String),
}

impl IdSalt {
    pub fn to_string(&self) -> String {
        match self {
            Self::Integer(val) => format!("{}", val),
            Self::String(s) => s.clone(),
        }
    }
}

impl From<i64> for IdSalt {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<String> for IdSalt {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl From<&str> for IdSalt {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<&String> for IdSalt {
    fn from(value: &String) -> Self {
        Self::String(value.clone())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LayoutDir {
    LeftToRight,
    TopToBottom,
}

#[derive(Debug, Clone, Copy)]
pub enum Size {
    Grow,
    Fit,
    Fixed(f32),
}

impl Size {
    fn as_fixed(&self) -> Option<f32> {
        match self {
            Size::Fixed(s) => Some(*s),
            _ => None,
        }
    }

    fn is_grow(&self) -> bool {
        match self {
            Size::Grow => true,
            _ => false,
        }
    }

    fn is_fit(&self) -> bool {
        match self {
            Size::Fit => true,
            _ => false,
        }
    }

    fn is_fixed(&self) -> bool {
        match self {
            Size::Fixed(_) => true,
            _ => false,
        }
    }
}

impl Into<Size> for f32 {
    fn into(self) -> Size {
        Size::Fixed(self)
    }
}

impl Into<Size> for u32 {
    fn into(self) -> Size {
        Size::Fixed(self as f32)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum TextJustify {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy)]
pub struct NodeStyle {
    bary_ui: LayoutDir,
    child_gap: f32,
    padding: f32,
    visible: bool,
    enabled_color: [f32; 4],
    disabled_color: [f32; 4],
    text_justify: TextJustify,
}

pub trait UiMsg: std::fmt::Debug + Clone + PartialEq + Eq {}

impl<T: std::fmt::Debug + Clone + PartialEq + Eq> UiMsg for T {}

#[derive(Debug, Clone)]
pub enum NodeType<T: UiMsg> {
    Text(String),
    Button(String, T),
    Image(String),
    Spacer,
    DragHandle(T),
    ProgressBar(f32),
    Row(Vec<Node<T>>),
    Column(Vec<Node<T>>),
}

#[derive(Debug, Clone)]
pub struct Node<T: UiMsg> {
    desired_width: Size,
    desired_height: Size,
    calculated_width: Option<f32>,
    calculated_height: Option<f32>,
    calculated_position: Option<Vec2>,
    layer: Option<u32>,
    // children: Vec<Node<T>>,
    node_type: NodeType<T>,
    enabled: bool,
    sprite: Option<String>,
    style: NodeStyle,
    path: NodePath,
    id_salt: IdSalt,
}

impl<T: UiMsg> Node<T> {
    pub fn new(
        width: impl Into<Size>,
        height: impl Into<Size>,
        id_salt: impl Into<IdSalt>,
    ) -> Self {
        let w = width.into();
        let h = height.into();
        Node {
            desired_width: w,
            desired_height: h,
            calculated_width: w.as_fixed(),
            calculated_height: h.as_fixed(),
            calculated_position: None,
            layer: None,
            node_type: NodeType::Spacer,
            enabled: true,
            sprite: None,
            style: NodeStyle {
                bary_ui: LayoutDir::LeftToRight,
                child_gap: 10.0,
                padding: 10.0,
                visible: true,
                enabled_color: [0.6, 0.3, 0.0, 0.8],
                disabled_color: [0.2, 0.2, 0.2, 0.8],
                text_justify: TextJustify::Center,
            },
            path: NodePath::new(),
            id_salt: id_salt.into(),
        }
    }

    pub fn text(width: impl Into<Size>, height: impl Into<Size>, text: impl Into<String>) -> Self {
        let text = text.into();
        let mut node = Self::new(width, height, &text);
        node.node_type = NodeType::Text(text);
        node
    }

    pub fn handle(
        width: impl Into<Size>,
        height: impl Into<Size>,
        id_salt: impl Into<IdSalt>,
        onclick: impl Into<T>,
    ) -> Self {
        let mut node = Self::new(width, height, id_salt);
        node.node_type = NodeType::DragHandle(onclick.into());
        node
    }

    pub fn structural(
        width: impl Into<Size>,
        height: impl Into<Size>,
        id_salt: impl Into<IdSalt>,
    ) -> Self {
        Self::new(width, height, id_salt)
    }

    pub fn image(
        sprite_name: impl Into<String>,
        width: impl Into<Size>,
        height: impl Into<Size>,
    ) -> Self {
        let name = sprite_name.into();
        let mut s = Self::new(width, height, &name);
        s.node_type = NodeType::Image(name);
        s
    }

    pub fn root(
        width: impl Into<Size>,
        height: impl Into<Size>,
        id_salt: impl Into<IdSalt>,
    ) -> Self {
        let mut node = Self::new(width, height, id_salt);
        node.node_type = NodeType::Column(vec![]);
        node
    }

    pub fn grow(id_salt: impl Into<IdSalt>) -> Self {
        Node::new(Size::Grow, Size::Grow, id_salt)
    }

    pub fn fit(id_salt: impl Into<IdSalt>) -> Self {
        Node::new(Size::Fit, Size::Fit, id_salt)
    }

    pub fn row(
        height: impl Into<Size>,
        id_salt: impl Into<IdSalt>,
        children: Vec<Node<T>>,
    ) -> Self {
        let mut node = Node::new(Size::Grow, height, id_salt).right();
        node.node_type = NodeType::Row(children);
        node
    }

    pub fn button(
        s: impl Into<String>,
        onclick: impl Into<T>,
        width: impl Into<Size>,
        height: impl Into<Size>,
        id_salt: impl Into<IdSalt>,
    ) -> Self {
        let mut node = Node::<T>::new(width, height, id_salt);
        node.node_type = NodeType::Button(s.into(), onclick.into());
        node
    }

    pub fn column(
        width: impl Into<Size>,
        id_salt: impl Into<IdSalt>,
        children: Vec<Node<T>>,
    ) -> Self {
        let mut node = Node::new(width, Size::Fit, id_salt).down();
        node.node_type = NodeType::Column(children);
        node
    }

    pub fn progress_bar(
        width: impl Into<Size>,
        height: impl Into<Size>,
        val: f32,
        id_salt: impl Into<IdSalt>,
    ) -> Self {
        let mut s = Node::new(width, height, id_salt);
        s.node_type = NodeType::ProgressBar(val);
        s
    }

    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub fn text_content(&self) -> Option<&String> {
        match &self.node_type {
            NodeType::Button(s, _) => Some(s),
            NodeType::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn with_sprite(mut self, s: impl Into<String>) -> Self {
        self.sprite = Some(s.into());
        self
    }

    pub fn sprite(&self) -> Option<&str> {
        self.sprite.as_ref().map(|s| s.as_str())
    }

    pub fn with_justify(mut self, s: TextJustify) -> Self {
        self.style.text_justify = s;
        self
    }

    pub fn justify(&self) -> TextJustify {
        self.style.text_justify
    }

    pub fn grid(
        width: impl Into<Size>,
        height: impl Into<Size>,
        rows: u32,
        cols: u32,
        spacing: f32,
        func: impl Fn(u32) -> Option<Node<T>>,
    ) -> Self {
        let mut i = 0;
        let mut root = Node::new(width, height, "grid_root")
            .invisible()
            .with_padding(0.0)
            .with_child_gap(spacing)
            .down();

        for r in 0..rows {
            let mut node = Node::grow("grid_row")
                .with_padding(0.0)
                .with_child_gap(spacing)
                .invisible();
            for c in 0..cols {
                let n = match func(i) {
                    Some(n) => n,
                    None => Node::grow("grid_col").enabled(false).invisible(),
                };
                i += 1;
                node.add_child(n);
            }
            root.add_child(node);
        }

        root
    }

    pub fn path(&self) -> &NodePath {
        &self.path
    }

    pub fn on_click(&self) -> Option<&T> {
        match &self.node_type {
            NodeType::Button(_, onclick) => Some(onclick),
            NodeType::DragHandle(onclick) => Some(onclick),
            _ => None,
        }
    }

    pub fn is_button(&self) -> bool {
        matches!(self.node_type, NodeType::Button(_, _))
    }

    pub fn with_layout(mut self, bary_ui: LayoutDir) -> Self {
        self.style.bary_ui = bary_ui;
        self
    }

    pub fn with_color(mut self, color: [f32; 4]) -> Self {
        self.style.enabled_color = color;
        self
    }

    pub fn set_color(&mut self, color: [f32; 4]) {
        self.style.enabled_color = color;
    }

    pub fn right(mut self) -> Self {
        self.style.bary_ui = LayoutDir::LeftToRight;
        self
    }

    pub fn down(mut self) -> Self {
        self.style.bary_ui = LayoutDir::TopToBottom;
        self
    }

    pub fn invisible(mut self) -> Self {
        self.style.visible = false;
        self
    }

    pub fn with_child_gap(mut self, child_gap: f32) -> Self {
        self.style.child_gap = child_gap;
        self
    }

    pub fn with_padding(mut self, padding: f32) -> Self {
        self.style.padding = padding;
        self
    }

    pub fn with_spacing(mut self, spacing: f32) -> Self {
        self.style.padding = spacing;
        self.style.child_gap = spacing;
        self
    }

    pub fn tight(mut self) -> Self {
        self.style.padding = 0.0;
        self.style.child_gap = 0.0;
        self
    }

    pub fn with_child(mut self, n: impl Into<Option<Node<T>>>) -> Self {
        if let Some(n) = n.into() {
            self.add_child(n);
        }
        self
    }

    pub fn with_children(mut self, nodes: impl Iterator<Item = Node<T>>) -> Self {
        nodes.for_each(|n| {
            self.add_child(n);
        });
        self
    }

    pub fn children_count(&self) -> usize {
        self.children().map(|e| e.len()).unwrap_or_default()
    }

    pub fn children(&self) -> Option<&Vec<Node<T>>> {
        match &self.node_type {
            NodeType::Column(children) => Some(children),
            NodeType::Row(children) => Some(children),
            _ => None,
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<Node<T>>> {
        match &mut self.node_type {
            NodeType::Column(children) => Some(children),
            NodeType::Row(children) => Some(children),
            _ => None,
        }
    }

    pub fn is_leaf(&self) -> bool {
        self.children().map(|e| e.is_empty()).unwrap_or(true)
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn is_visible(&self) -> bool {
        self.style.visible
    }

    pub fn kind(&self) -> &NodeType<T> {
        &self.node_type
    }

    pub fn add_child(&mut self, n: Node<T>) -> &mut Self {
        if let Some(c) = self.children_mut() {
            c.push(n);
        } else {
            warn!("Failed to add child to node with type {:?}", self.node_type);
        }
        self
    }

    pub fn add_children(&mut self, nodes: impl Iterator<Item = Node<T>>) -> &mut Self {
        nodes.for_each(|n| {
            self.add_child(n);
        });
        self
    }

    pub fn layer(&self) -> u32 {
        self.layer.unwrap_or(0)
    }

    pub fn color(&self) -> [f32; 4] {
        if self.enabled {
            self.style.enabled_color
        } else {
            self.style.disabled_color
        }
    }

    pub fn color_u8(&self) -> [u8; 4] {
        let [r, g, b, a] = self.color();
        [
            (r * 255.0).round() as u8,
            (g * 255.0).round() as u8,
            (b * 255.0).round() as u8,
            (a * 255.0).round() as u8,
        ]
    }

    pub fn desired_dims(&self) -> (Size, Size) {
        (self.desired_width, self.desired_height)
    }

    pub fn fixed_dims(&self) -> Vec2 {
        Vec2::new(
            self.desired_width.as_fixed().unwrap_or(0.0),
            self.desired_height.as_fixed().unwrap_or(0.0),
        )
    }

    pub fn calculated_dims(&self) -> Vec2 {
        Vec2::new(
            self.calculated_width.unwrap_or(0.0),
            self.calculated_height.unwrap_or(0.0),
        )
    }

    pub fn aabb(&self) -> AABB {
        let a = self.calculated_position.unwrap_or(Vec2::ZERO);
        let b = a + self.calculated_dims();
        AABB::from_arbitrary(a, b)
    }

    pub fn aabb_camera(&self, wb: Vec2) -> AABB {
        let aabb = self.aabb();
        let offset = Vec2::new(-wb.x / 2.0, wb.y / 2.0);
        aabb.flip_y_about(0.0).offset(offset)
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<T>> + use<'_, T> {
        let self_iter = [self].into_iter();
        let child_iters: Vec<&Node<T>> = self
            .children()
            .iter()
            .flat_map(|n| n.iter().flat_map(|e| e.iter()))
            .collect::<Vec<_>>();
        self_iter.chain(child_iters)
    }
}

fn sum_fixed_dims<'a, T: 'a + UiMsg>(
    bary_ui: LayoutDir,
    nodes: impl Iterator<Item = &'a Node<T>>,
    padding: f32,
    childgap: f32,
) -> Vec2 {
    let mut sx: f32 = 0.0;
    let mut sy: f32 = 0.0;

    for node in nodes {
        let dims = node.fixed_dims();
        match bary_ui {
            LayoutDir::LeftToRight => {
                sx += dims.x + childgap;
                sy = sy.max(dims.y);
            }
            LayoutDir::TopToBottom => {
                sx = sx.max(dims.x);
                sy += dims.y + childgap;
            }
        };
    }

    if sx > 0.0 {
        match bary_ui {
            LayoutDir::LeftToRight => sx -= childgap,
            _ => (),
        }
    }

    if sy > 0.0 {
        match bary_ui {
            LayoutDir::TopToBottom => sy -= childgap,
            _ => (),
        }
    }

    sx += padding * 2.0;
    sy += padding * 2.0;

    Vec2::new(sx, sy)
}

#[test]
fn simple_col_layout() {
    let a = Node::button("Hello", "wow", 300, 60, "");
    let b = Node::button("Hello", "wow", 300, 60, "");
    let c = Node::button("Hello", "wow", 300, 60, "");

    // let nested_col = Node::column(
    //     Size::Fit,
    //     vec![
    //         // Node::button("Hello", "wow", 300, 60),
    //         // Node::button("Hello", "wow", 300, 60),
    //         // Node::button("Hello", "wow", 300, 60),
    //     ],
    // );

    let children = vec![a, b, c];

    let dims = sum_fixed_dims(LayoutDir::TopToBottom, children.iter(), 0.0, 0.0);

    assert_eq!(dims, Vec2::new(300.0, 180.0));

    let root = Node::<String>::column(Size::Fit, "", children);
}

fn populate_positions<'a, T: 'a + UiMsg>(mut root: &mut Node<T>, origin: impl Into<Option<Vec2>>) {
    let origin = origin.into().unwrap_or(Vec2::ZERO);
    root.calculated_position = Some(origin);

    let mut px = origin.x + root.style.padding;
    let mut py = origin.y + root.style.padding;

    let layout = root.style.bary_ui;
    let child_gap = root.style.child_gap;

    root.children_mut().iter_mut().for_each(|e| {
        e.iter_mut().for_each(|n| {
            let dim = n.calculated_dims();
            let o = Vec2::new(px, py);
            match layout {
                LayoutDir::LeftToRight => px += dim.x + child_gap,
                LayoutDir::TopToBottom => py += dim.y + child_gap,
            }
            populate_positions(n, o)
        })
    });
}

fn assign_layers_and_paths<T: UiMsg>(root: &mut Node<T>, path: NodePath, layer: u32) {
    root.layer = Some(layer);
    root.path = path.clone();

    if let Some(c) = root.children_mut() {
        for c in c {
            let child_path = path.append(c.id_salt.to_string());
            assign_layers_and_paths(c, child_path, layer + 1);
        }
    }
}

pub fn populate_fit_sizes<T: UiMsg>(root: &mut Node<T>) {
    if root.is_leaf() {
        if root.desired_width.is_fit() {
            root.calculated_width = Some(25.0);
        }
        if root.desired_height.is_fit() {
            root.calculated_height = Some(25.0);
        }
        return;
    }

    root.children_mut()
        .iter_mut()
        .for_each(|n| n.iter_mut().for_each(|n| populate_fit_sizes(n)));

    let dims = sum_fixed_dims(
        root.style.bary_ui,
        root.children().iter().flat_map(|e| e.iter()),
        root.style.padding,
        root.style.child_gap,
    );

    if root.desired_width.is_fit() {
        root.calculated_width = Some(dims.x);
    }

    if root.desired_height.is_fit() {
        root.calculated_height = Some(dims.y);
    }
}

pub fn populate_grow_sizes<T: UiMsg>(root: &mut Node<T>) {
    if root.is_leaf() {
        return;
    }

    let mut n_to_grow = 0;

    if let Some(children) = root.children() {
        for c in children {
            let dim = match root.style.bary_ui {
                LayoutDir::LeftToRight => c.desired_width.is_grow(),
                LayoutDir::TopToBottom => c.desired_height.is_grow(),
            } as u32;
            n_to_grow += dim;
        }
    }

    let mut w = root.calculated_width.unwrap_or(0.0) - root.style.padding * 2.0;
    let mut h = root.calculated_height.unwrap_or(0.0) - root.style.padding * 2.0;

    let layout = root.style.bary_ui;
    let child_gap = root.style.child_gap;

    if let Some(children) = root.children_mut() {
        for c in children {
            match layout {
                LayoutDir::LeftToRight => w -= (c.calculated_width.unwrap_or(0.0) + child_gap),
                LayoutDir::TopToBottom => h -= (c.calculated_height.unwrap_or(0.0) + child_gap),
            }
        }
    }

    let n_to_grow = n_to_grow.max(1);

    match root.style.bary_ui {
        LayoutDir::LeftToRight => {
            w += root.style.child_gap;
            w /= n_to_grow as f32;
        }
        LayoutDir::TopToBottom => {
            h += root.style.child_gap;
            h /= n_to_grow as f32;
        }
    }

    root.children_mut().iter_mut().for_each(|mut c| {
        c.iter_mut().for_each(|c| {
            if c.desired_width.is_grow() {
                c.calculated_width = Some(w);
            }
            if c.desired_height.is_grow() {
                c.calculated_height = Some(h);
            }
            populate_grow_sizes(c)
        })
    });
}

#[derive(Debug, Clone)]
pub struct Tree<T: UiMsg> {
    roots: Vec<Node<T>>,
}

impl<T: UiMsg> Tree<T> {
    pub fn new() -> Tree<T> {
        Tree { roots: Vec::new() }
    }

    pub fn add_layout(&mut self, mut node: Node<T>, origin: impl Into<Option<Vec2>>) {
        let origin = origin.into().unwrap_or(Vec2::ZERO);
        populate_fit_sizes(&mut node);
        populate_grow_sizes(&mut node);
        populate_positions(&mut node, origin);
        assign_layers_and_paths(&mut node, NodePath::new(), 0);
        self.roots.push(node);
    }

    pub fn with_layout(mut self, node: Node<T>, origin: impl Into<Option<Vec2>>) -> Self {
        self.add_layout(node, origin);
        self
    }

    pub fn layouts(&self) -> &Vec<Node<T>> {
        &self.roots
    }

    pub fn iter(&self) -> impl Iterator<Item = &Node<T>> {
        self.roots.iter().flat_map(|e| e.iter())
    }

    pub fn at(&self, p: Vec2, wb: Vec2) -> Option<&Node<T>> {
        for bary_ui in self.roots.iter().rev() {
            let mut candidates: Vec<&Node<T>> = bary_ui
                .iter()
                .filter(|n| n.aabb_camera(wb).contains(p))
                .filter(|n| n.is_visible())
                .collect();
            if candidates.is_empty() {
                continue;
            }
            candidates.sort_by_key(|n| n.layer());
            return candidates.last().map(|v| *v);
        }
        None
    }
}

fn write_node<'a, T: UiMsg>(
    f: &mut std::fmt::Formatter<'a>,
    node: &Node<T>,
    level: usize,
) -> std::fmt::Result {
    let spacer: String = (0..level * 2).map(|_| ' ').collect();
    let repr = match &node.node_type {
        NodeType::Text(_) => "Text",
        NodeType::Button(_, _) => "Button",
        NodeType::Image(_) => "Image",
        NodeType::Spacer => "Spacer",
        NodeType::DragHandle(_) => "DragHandle",
        NodeType::ProgressBar(_) => "ProgressBar",
        NodeType::Row(nodes) => "Row",
        NodeType::Column(nodes) => "Column",
    };
    write!(
        f,
        "{}[{:?} pos={:?} dims={}]\n",
        spacer,
        repr,
        node.calculated_position,
        node.calculated_dims()
    )?;

    if let Some(children) = node.children() {
        for c in children {
            write_node(f, c, level + 1)?;
        }
    }
    Ok(())
}

impl<T: UiMsg> std::fmt::Display for Node<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write_node(f, self, 0)
    }
}

impl<T: UiMsg> std::fmt::Display for Tree<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for node in self.layouts() {
            write_node(f, node, 0)?;
        }
        Ok(())
    }
}

pub fn write_layout_to_svg<T: UiMsg>(filepath: &str, tree: &Tree<T>) -> Result<(), std::io::Error> {
    let aabbs: Vec<(AABB, [f32; 4])> = tree
        .layouts()
        .iter()
        .flat_map(|r| r.iter().map(|n| n).collect::<Vec<_>>())
        .filter_map(|n| n.is_visible().then(|| (n.aabb(), n.color())))
        .collect();

    write_svg(filepath, &aabbs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::write_svg;

    #[test]
    fn write_layout() {
        let tree = crate::examples::example_layout(1700.0, 1200.0);
        write_layout_to_svg("example_layout.svg", &tree).unwrap();
    }

    #[test]
    fn fixed_dims() {
        let a = Node::new(300.0, 700.0, "");
        let b = Node::new(200.0, 400.0, "");
        let c = Node::new(550.0, 300.0, "");

        let nodes = [&a, &b, &c];

        let l2r = sum_fixed_dims(LayoutDir::LeftToRight, nodes.into_iter(), 0.0, 0.0);
        let t2b = sum_fixed_dims(LayoutDir::TopToBottom, nodes.into_iter(), 0.0, 0.0);

        assert_eq!(l2r.x, 1050.0);
        assert_eq!(l2r.y, 700.0);

        assert_eq!(t2b.x, 550.0);
        assert_eq!(t2b.y, 1400.0);

        let l2r = sum_fixed_dims(LayoutDir::LeftToRight, nodes.into_iter(), 12.0, 7.5);
        let t2b = sum_fixed_dims(LayoutDir::TopToBottom, nodes.into_iter(), 12.0, 7.5);

        assert_eq!(l2r.x, 1089.0);
        assert_eq!(l2r.y, 724.0);

        assert_eq!(t2b.x, 574.0);
        assert_eq!(t2b.y, 1439.0);

        let mut root = Node::<String>::root(Size::Fit, Size::Fit, "")
            .with_child(a)
            .with_child(b)
            .with_child(c);

        populate_fit_sizes(&mut root);
        populate_grow_sizes(&mut root);
        populate_positions(&mut root, None);
        assign_layers_and_paths(&mut root, NodePath::new(), 0);

        let aabbs = root
            .iter()
            .map(|n| (n.aabb(), n.color()))
            .collect::<Vec<_>>();
        write_svg("boxes.svg", &aabbs).unwrap();

        let dims = root.calculated_dims();

        assert_eq!(dims.x, 1090.0);
        assert_eq!(dims.y, 720.0);
    }
}
