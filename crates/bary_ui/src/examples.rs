use crate::layout::*;

fn box_with_corners(w: f32) -> Node<String> {
    let banner = || {
        Node::row(Size::Fit, "box_with_corners", vec![])
            .invisible()
            .with_child(Node::structural(w, w, "s1"))
            .with_child(Node::grow("g1").invisible())
            .with_child(Node::structural(w, w, "s2"))
            .with_child(Node::grow("g2").invisible())
            .with_child(Node::structural(w, w, "s3"))
    };

    Node::grow("g3")
        .invisible()
        .tight()
        .down()
        .with_child(banner())
        .with_child(Node::grow("g4").invisible())
        .with_child(banner())
}

#[allow(unused)]
fn text_dims(s: &str) -> (usize, usize) {
    let max_line = s.lines().map(|l| l.len()).max().unwrap_or(0);
    let lines = s.lines().count();
    (lines, max_line)
}

#[allow(unused)]
fn text_node(s: &str, width: impl Into<Size>, height: impl Into<Size>) -> Node<String> {
    let chr_width = 15.0;
    let chr_height = 30.0;
    let (lines, max_line) = text_dims(&s);
    let twidth = max_line as f32 * chr_width;
    let theight = lines as f32 * chr_height;
    Node::structural(width, height, "")
        .tight()
        .down()
        .with_child(
            Node::grow("")
                .tight()
                .with_child(Node::grow(""))
                .with_child(
                    Node::column(
                        Size::Fit,
                        "",
                        vec![
                            Node::grow(""),
                            Node::text(twidth, theight, s),
                            Node::grow(""),
                        ],
                    )
                    .tight(),
                )
                .with_child(Node::grow("")),
        )
}

pub fn example_layout(width: f32, height: f32) -> Tree<String> {
    let spacing = 8.0;

    let a = Node::button(
        "wow this is\na fair amount\nof text",
        "dingus",
        Size::Grow,
        Size::Grow,
        "button",
    );

    let b = Node::grid(Size::Grow, Size::Grow, 2, 4, spacing, |_| {
        Some(Node::grow("grow"))
    });

    // let c = (0..6).map(|i| {
    //     let a = Node::structural(40 + i * 6, 10);
    //     let b = Node::grow();
    //     Node::row(Size::Fit, vec![a, b])
    //         .invisible()
    //         .with_padding(0.0);
    // });

    let g = Node::grid(Size::Grow, 100, 4, 5, spacing, |_| Some(Node::grow("")));

    let d = Node::row(30, "", vec![]);

    let sidebar = Node::column(300.0, "", vec![a, b, g, d])
        .with_spacing(spacing)
        .with_children((0..4).map(|_| Node::row(25, "", vec![])));

    let topbar = Node::row(Size::Fit, "", vec![])
        .with_spacing(spacing)
        .with_children((0..10).map(|i| Node::text(120, 40, &format!("thing {}", i))))
        .with_children((0..5).map(|_| Node::grow("").invisible()))
        .with_child(Node::column(70, "", vec![]));

    let main = Node::grow("")
        .tight()
        .invisible()
        .with_child(sidebar)
        .with_child(
            Node::grow("")
                .invisible()
                .down()
                .tight()
                .with_children([box_with_corners(40.0), Node::row(30, "", vec![])].into_iter()),
        );

    let root = Node::root(width, height, "")
        .invisible()
        .tight()
        .down()
        .with_child(topbar)
        .with_child(main);

    Tree::new().with_layout(root, None)
}
