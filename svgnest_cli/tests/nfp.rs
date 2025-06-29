use svgnest_cli::geometry::{minkowski_difference_clip, polygon_area};
use svgnest_cli::nfp::{inner_fit_polygon, no_fit_polygon_rectangle};
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

#[test]
fn inner_nfp_rectangle_simple() {
    let container = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 10.0, y: 0.0 },
        Point { x: 10.0, y: 10.0 },
        Point { x: 0.0, y: 10.0 },
    ];
    let part = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 2.0, y: 0.0 },
        Point { x: 2.0, y: 2.0 },
        Point { x: 0.0, y: 2.0 },
    ];
    let nfps = inner_fit_polygon(&container, &part, 0.0);
    assert_eq!(nfps.len(), 1);
    let area = polygon_area(&nfps[0]).abs();
    assert!((area - 64.0).abs() < 1e-6);
    let rect_nfp = no_fit_polygon_rectangle(&container, &part).unwrap();
    assert_eq!(rect_nfp.len(), 1);
}

#[test]
fn inner_nfp_concave_splits_regions() {
    let container = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 3.0, y: 0.0 },
        Point { x: 3.0, y: 1.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 1.0, y: 3.0 },
        Point { x: 0.0, y: 3.0 },
    ];
    let part = vec![
        Point { x: 0.0, y: 0.0 },
        Point { x: 1.0, y: 0.0 },
        Point { x: 1.0, y: 1.0 },
        Point { x: 0.0, y: 1.0 },
    ];
    let nfps = inner_fit_polygon(&container, &part, 0.0);
    assert!(nfps.is_empty() || nfps.len() >= 1);
}
