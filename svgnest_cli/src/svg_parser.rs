use roxmltree::{Document, Node};
use std::fs;
use std::path::Path;
use lyon_path::{iterator::PathIterator, Path as LyonPath, PathEvent};
use lyon_svg::path_utils::build_path;

/// Simple 2D transformation matrix represented as [a,b,c,d,e,f].
#[derive(Clone, Copy, Debug)]
struct Transform([f64; 6]);

impl Transform {
    fn identity() -> Self {
        Self([1.0, 0.0, 0.0, 1.0, 0.0, 0.0])
    }

    fn multiply(&self, other: &Self) -> Self {
        let m1 = self.0;
        let m2 = other.0;
        Self([
            m1[0] * m2[0] + m1[2] * m2[1],
            m1[1] * m2[0] + m1[3] * m2[1],
            m1[0] * m2[2] + m1[2] * m2[3],
            m1[1] * m2[2] + m1[3] * m2[3],
            m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
            m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
        ])
    }

    fn apply(&self, x: f64, y: f64) -> (f64, f64) {
        let m = self.0;
        (x * m[0] + y * m[2] + m[4], x * m[1] + y * m[3] + m[5])
    }
}

/// Parse a `transform` attribute into a [`Transform`].
fn parse_transform(value: &str) -> Transform {
    use std::str::FromStr;
    let mut result = Transform::identity();
    let tokens = value.split(|c| c == ')' || c == ',').collect::<Vec<_>>();
    for token in tokens {
        let trimmed = token.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("translate(") {
            let mut nums = rest.split_whitespace();
            let tx = nums
                .next()
                .and_then(|v| f64::from_str(v).ok())
                .unwrap_or(0.0);
            let ty = nums
                .next()
                .and_then(|v| f64::from_str(v).ok())
                .unwrap_or(0.0);
            let t = Transform([1.0, 0.0, 0.0, 1.0, tx, ty]);
            result = result.multiply(&t);
        } else if let Some(rest) = trimmed.strip_prefix("scale(") {
            let mut nums = rest.split_whitespace();
            let sx = nums
                .next()
                .and_then(|v| f64::from_str(v).ok())
                .unwrap_or(1.0);
            let sy = nums
                .next()
                .and_then(|v| f64::from_str(v).ok())
                .unwrap_or(sx);
            let t = Transform([sx, 0.0, 0.0, sy, 0.0, 0.0]);
            result = result.multiply(&t);
        } else if let Some(rest) = trimmed.strip_prefix("rotate(") {
            let nums: Vec<_> = rest.split_whitespace().collect();
            if let Ok(angle) = f64::from_str(nums.get(0).unwrap_or(&"0")) {
                let (sx, sy) = if nums.len() == 3 {
                    (
                        nums.get(1)
                            .and_then(|v| f64::from_str(v).ok())
                            .unwrap_or(0.0),
                        nums.get(2)
                            .and_then(|v| f64::from_str(v).ok())
                            .unwrap_or(0.0),
                    )
                } else {
                    (0.0, 0.0)
                };
                let rad = angle.to_radians();
                let cos = rad.cos();
                let sin = rad.sin();
                let rotation = Transform([cos, sin, -sin, cos, 0.0, 0.0]);
                let pre = Transform([1.0, 0.0, 0.0, 1.0, sx, sy]);
                let post = Transform([1.0, 0.0, 0.0, 1.0, -sx, -sy]);
                result = result.multiply(&pre).multiply(&rotation).multiply(&post);
            }
        } else if let Some(rest) = trimmed.strip_prefix("matrix(") {
            let nums: Vec<_> = rest.split_whitespace().collect();
            if nums.len() >= 6 {
                if let (Ok(a), Ok(b), Ok(c), Ok(d), Ok(e), Ok(f)) = (
                    f64::from_str(nums[0]),
                    f64::from_str(nums[1]),
                    f64::from_str(nums[2]),
                    f64::from_str(nums[3]),
                    f64::from_str(nums[4]),
                    f64::from_str(nums[5]),
                ) {
                    result = result.multiply(&Transform([a, b, c, d, e, f]));
                }
            }
        }
    }
    result
}

/// Single point.
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

/// Polygon composed of points.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Polygon {
    /// Unique identifier assigned during parsing
    pub id: usize,
    /// Vertices of the polygon
    pub points: Vec<Point>,
    /// Whether the polygon forms a closed path
    pub closed: bool,
}

/// Approximate a SVG path into points using recursive subdivision with the given tolerance.
pub fn approximate_path(d: &str, tol: f64) -> Vec<(bool, Vec<(f64, f64)>)> {
    let builder = LyonPath::builder().with_svg();
    let path = match build_path(builder, d) {
        Ok(p) => p,
        Err(_) => return Vec::new(),
    };
    let mut result = Vec::new();
    let mut current = Vec::new();
    let mut closed = false;
    for evt in path.iter().flattened(tol as f32) {
        match evt {
            PathEvent::Begin { at } => {
                if !current.is_empty() {
                    result.push((closed, current));
                    current = Vec::new();
                }
                current.push((at.x as f64, at.y as f64));
                closed = false;
            }
            PathEvent::Line { to, .. } => {
                current.push((to.x as f64, to.y as f64));
            }
            PathEvent::End { close, .. } => {
                closed = close;
            }
            _ => {}
        }
    }
    if !current.is_empty() {
        result.push((closed, current));
    }
    result
}

/// Parse an SVG file and return all polygons.
pub fn polygons_from_file(path: &Path, merge: bool, tol: f64) -> anyhow::Result<Vec<Polygon>> {
    let data = fs::read_to_string(path)?;
    polygons_from_str(&data, merge, tol)
}

/// Parse an SVG string and return all polygons.
pub fn polygons_from_str(data: &str, merge: bool, tol: f64) -> anyhow::Result<Vec<Polygon>> {
    let doc = Document::parse(data)?;
    let root = doc.root_element();
    let mut polys = Vec::new();
    extract_node_polygons(root, Transform::identity(), tol, &mut polys)?;
    for (i, p) in polys.iter_mut().enumerate() {
        p.id = i;
    }
    if merge {
        Ok(crate::line_merge::merge_lines(&polys))
    } else {
        Ok(polys)
    }
}

fn extract_node_polygons(
    node: Node,
    transform: Transform,
    tol: f64,
    output: &mut Vec<Polygon>,
) -> anyhow::Result<()> {
    let node_transform = node
        .attribute("transform")
        .map(parse_transform)
        .unwrap_or(Transform::identity());
    let transform = transform.multiply(&node_transform);

    match node.tag_name().name() {
        "path" => {
            if let Some(d) = node.attribute("d") {
                for (closed, pts) in approximate_path(d, tol) {
                    let mapped = pts
                        .into_iter()
                        .map(|(x, y)| {
                            let (x, y) = transform.apply(x, y);
                            Point { x, y }
                        })
                        .collect();
                    output.push(Polygon {
                        id: 0,
                        points: mapped,
                        closed,
                    });
                }
            }
        }
        "polygon" | "polyline" => {
            if let Some(points_str) = node.attribute("points") {
                let mut pts = Vec::new();
                for pair in points_str.split_whitespace() {
                    let mut nums = pair.split(',');
                    if let (Some(x), Some(y)) = (nums.next(), nums.next()) {
                        if let (Ok(x), Ok(y)) = (x.parse::<f64>(), y.parse::<f64>()) {
                            let (x, y) = transform.apply(x, y);
                            pts.push(Point { x, y });
                        }
                    }
                }
                output.push(Polygon {
                    id: 0,
                    points: pts,
                    closed: node.tag_name().name() == "polygon",
                });
            }
        }
        "rect" => {
            let x = node
                .attribute("x")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let y = node
                .attribute("y")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let w = node
                .attribute("width")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let h = node
                .attribute("height")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let pts = vec![
                Point { x, y },
                Point { x: x + w, y },
                Point { x: x + w, y: y + h },
                Point { x, y: y + h },
            ];
            let pts: Vec<_> = pts
                .into_iter()
                .map(|p| {
                    let (x, y) = transform.apply(p.x, p.y);
                    Point { x, y }
                })
                .collect();
            output.push(Polygon {
                id: 0,
                points: pts,
                closed: true,
            });
        }
        "circle" => {
            let cx = node
                .attribute("cx")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let cy = node
                .attribute("cy")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let r = node
                .attribute("r")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let segments = 32;
            let mut pts = Vec::new();
            for i in 0..segments {
                let theta = i as f64 * std::f64::consts::TAU / segments as f64;
                let (x, y) = (cx + r * theta.cos(), cy + r * theta.sin());
                let (x, y) = transform.apply(x, y);
                pts.push(Point { x, y });
            }
            output.push(Polygon {
                id: 0,
                points: pts,
                closed: true,
            });
        }
        "ellipse" => {
            let cx = node
                .attribute("cx")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let cy = node
                .attribute("cy")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let rx = node
                .attribute("rx")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let ry = node
                .attribute("ry")
                .unwrap_or("0")
                .parse::<f64>()
                .unwrap_or(0.0);
            let segments = 32;
            let mut pts = Vec::new();
            for i in 0..segments {
                let theta = i as f64 * std::f64::consts::TAU / segments as f64;
                let (x, y) = (cx + rx * theta.cos(), cy + ry * theta.sin());
                let (x, y) = transform.apply(x, y);
                pts.push(Point { x, y });
            }
            output.push(Polygon {
                id: 0,
                points: pts,
                closed: true,
            });
        }
        "line" => {
            if let (Some(x1), Some(y1), Some(x2), Some(y2)) = (
                node.attribute("x1"),
                node.attribute("y1"),
                node.attribute("x2"),
                node.attribute("y2"),
            ) {
                if let (Ok(x1), Ok(y1), Ok(x2), Ok(y2)) = (
                    x1.parse::<f64>(),
                    y1.parse::<f64>(),
                    x2.parse::<f64>(),
                    y2.parse::<f64>(),
                ) {
                    let (x1, y1) = transform.apply(x1, y1);
                    let (x2, y2) = transform.apply(x2, y2);
                    output.push(Polygon {
                        id: 0,
                        points: vec![Point { x: x1, y: y1 }, Point { x: x2, y: y2 }],
                        closed: false,
                    });
                }
            }
        }
        _ => {}
    }

    for child in node.children().filter(|n| n.is_element()) {
        extract_node_polygons(child, transform, tol, output)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_rect() {
        let svg = r#"<svg><rect x="0" y="0" width="10" height="10"/></svg>"#;
        let polys = polygons_from_str(svg, false, crate::geometry::CURVE_TOLERANCE).unwrap();
        assert_eq!(polys.len(), 1);
        assert_eq!(polys[0].points.len(), 4);
    }

    #[test]
    fn merge_lines_option() {
        let svg = "<svg><line x1='0' y1='0' x2='1' y2='0'/><line x1='1' y1='0' x2='0' y2='0'/></svg>";
        let polys = polygons_from_str(svg, true, crate::geometry::CURVE_TOLERANCE).unwrap();
        assert_eq!(polys.len(), 1);
    }

    #[test]
    fn approximate_arc_accuracy() {
        let d = "M0,0 A10,10 0 0 1 10,0";
        let paths = approximate_path(d, 0.1);
        assert_eq!(paths.len(), 1);
        let (_closed, pts) = &paths[0];
        let center = (5.0f64, 8.660254037844386f64);
        for (x, y) in pts {
            let r = ((x - center.0).powi(2) + (y - center.1).powi(2)).sqrt();
            println!("pt: ({},{}), r diff: {}", x, y, (r - 10.0).abs());
            assert!((r - 10.0).abs() <= 0.1 + 1e-6);
        }
    }

    #[test]
    fn approximate_matches_lyon() {
        let d = "M0,0 C0,10 10,10 10,0";
        let tol = 0.05;
        let ours = &approximate_path(d, tol)[0].1;

        let builder = LyonPath::builder().with_svg();
        let path = build_path(builder, d).unwrap();
        let mut expected = Vec::new();
        for evt in path.iter().flattened(tol as f32) {
            match evt {
                PathEvent::Begin { at } => expected.push((at.x as f64, at.y as f64)),
                PathEvent::Line { to, .. } => expected.push((to.x as f64, to.y as f64)),
                _ => {}
            }
        }

        assert_eq!(*ours, expected);
    }
}
