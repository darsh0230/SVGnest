use svgnest_cli::geometry::{minkowski_difference_clip, polygon_area};
use svgnest_cli::svg_parser::Point;

#[test]
fn concave_minkowski_handles_l_shape() {
    let a = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 2.0, y: 0.0 },
        Point { x: 2.0, y: 1.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: 2.0 },
        Point { x: 0.0, y: 2.0 },
    ];
    let b = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 0.0, y: 1.0 },
    ];
    let nfp = minkowski_difference_clip(&a, &b);
    assert!(nfp.len() > 4);
    let area = polygon_area(&nfp).abs();
    println!("area: {}", area);
    assert!((area - 5.0).abs() < 0.1);
}
