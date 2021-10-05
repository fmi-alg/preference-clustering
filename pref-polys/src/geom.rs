use crate::{float_eq, utils::MyVec};

#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub enum Orientation {
    Inside,
    Outside,
}

pub fn orientation_test(constraint: &[f64], preference: &[f64]) -> Orientation {
    assert_eq!(constraint.len(), preference.len() + 1);

    let last = *constraint.last().unwrap();
    let sum: f64 = constraint.iter().zip(preference).map(|(c, p)| c * p).sum();
    if sum >= last || float_eq!(sum, last) {
        Orientation::Inside
    } else {
        Orientation::Outside
    }
}

pub fn center_point(points: &[MyVec<f64>]) -> MyVec<f64> {
    let mut result = MyVec::from(vec![0.0; points[0].len()]);

    for p in points {
        p.iter().zip(result.iter_mut()).for_each(|(p, r)| *r += *p);
    }

    let point_count = points.len() as f64;

    result.iter_mut().for_each(|r| *r /= point_count);
    result
}

pub fn sort_points_ccw(points: &mut [MyVec<f64>]) {
    let center = center_point(points);
    assert_eq!(2, center.len(), "Sorting ccw only for dimension 2");

    points.sort_by_cached_key(|p| {
        let angle = angle(p, &center);
        ordered_float::OrderedFloat(angle)
    })
}

pub fn angle(p: &[f64], center: &[f64]) -> f64 {
    let x = p[0] - center[0];
    let y: f64 = p[1] - center[1];
    y.atan2(x)
}

pub fn intersection(a: &[f64], b: &[f64]) -> Option<MyVec<f64>> {
    assert_eq!(
        a.len(),
        3,
        "Intersection is only implemented for dimension 2 (len = 3)"
    );
    assert_eq!(a.len(), b.len());

    let y_denom = -a[0] * b[1] + b[0] * a[1];
    let y_num = a[2] * b[0] - a[0] * b[2];

    let y = if float_eq!(y_denom, 0.0) {
        return None;
    } else {
        y_num / y_denom
    };
    let x_num;
    let x_denom;

    if b[0] == 0.0 {
        x_num = a[2] - a[1] * y;
        x_denom = a[0];
    } else {
        x_num = b[2] - b[1] * y;
        x_denom = b[0];
    }
    let x = if float_eq!(x_denom, 0.0) {
        return None;
    } else {
        x_num / x_denom
    };

    Some(vec![x, y].into())
}

#[test]
fn test_orientation_test() {
    let constraint = [-3., 2., 0.];
    let alpha_inside = [0.2, 0.6];
    let orientation = orientation_test(&constraint, &alpha_inside);
    assert_eq!(Orientation::Inside, orientation);

    let alpha_outside = [0.4, 0.4];

    let orientation = orientation_test(&constraint, &alpha_outside);
    assert_eq!(Orientation::Outside, orientation);

    let alpha_on_top = [0.4, 0.6];

    let orientation = orientation_test(&constraint, &alpha_on_top);
    assert_eq!(Orientation::Inside, orientation);
}

#[cfg(test)]
mod tests {
    use super::*;

    use rand::prelude::{SliceRandom, ThreadRng};

    use crate::utils::same_array;

    #[test]
    fn test_center_point() {
        let points = vec![
            vec![0.06, 0.52].into(),
            vec![0.02, 0.09].into(),
            vec![0.48, 0.08].into(),
            vec![0.28, 0.45].into(),
            vec![0.42, 0.31].into(),
            vec![0.32, 0.06].into(),
        ];

        let center = center_point(&points);
        assert!(same_array(dbg!(&center), &[0.2633333333, 0.251666666]));

        let points = vec![
            vec![0.0, 1.0].into(),
            vec![0.02, 0.09].into(),
            vec![0.06, 0.05].into(),
            vec![0.11, 0.03].into(),
            vec![0.09, 0.04].into(),
            vec![0.04, 0.07].into(),
        ];

        let center = center_point(&points);
        assert!(same_array(dbg!(&center), &[0.0533333333, 0.213333333,]));
    }

    #[test]
    fn test_sort_ccw() {
        let sorted_points = vec![
            vec![0.02, 0.09].into(),
            vec![0.04, 0.07].into(),
            vec![0.06, 0.05].into(),
            vec![0.09, 0.04].into(),
            vec![0.11, 0.03].into(),
            vec![0.0, 1.0].into(),
        ];

        // already sorted case
        let mut points = sorted_points.clone();
        sort_points_ccw(&mut points);

        for (s, p) in sorted_points.iter().zip(&points) {
            assert!(same_array(dbg!(&s), dbg!(&p)));
        }

        let mut points = sorted_points.clone();
        let mut rng = ThreadRng::default();
        points.shuffle(&mut rng);

        sort_points_ccw(&mut points);

        for (s, p) in sorted_points.iter().zip(&points) {
            assert!(same_array(dbg!(&s), dbg!(&p)));
        }
    }
    #[test]
    fn test_intersection() {
        let a = [2.0, -3.0, 4.0];
        let b = [-3.0, 7.0, 2.0];
        let point = intersection(&a, &b).unwrap();

        assert!(same_array(dbg!(&point), &[6.8, 3.2]));

        assert_eq!(Orientation::Inside, orientation_test(&a, &point));
        assert_eq!(Orientation::Inside, orientation_test(&b, &point));
    }

    #[test]
    fn test_intersection_with_0_components() {
        let a = [28.0, 385.0, 41.0];
        let b = [0.0, 95.0, 4.0];
        let point = intersection(&a, &b).unwrap();

        assert!(same_array(
            dbg!(&point),
            &[0.885338345864, 0.042105263157894736]
        ));

        assert_eq!(Orientation::Inside, orientation_test(&a, &point));
        assert_eq!(Orientation::Inside, orientation_test(&b, &point));

        assert!(same_array(&point, &intersection(&b, &a).unwrap()));

        let a = [28.0, 385.0, 41.0];
        let b = [95.0, 0.0, 4.0];
        let point = intersection(&a, &b).unwrap();

        assert!(same_array(
            dbg!(&point),
            &[0.042105263157894736, 0.1034313055365687]
        ));

        assert!(same_array(&point, &intersection(&b, &a).unwrap()));

        assert_eq!(Orientation::Inside, orientation_test(&a, &point));
        assert_eq!(Orientation::Inside, orientation_test(&b, &point));
    }

    #[test]
    fn test_parallel_lines_do_not_intersect() {
        let a = [0.0, 1.0, 0.0];
        let b = [0.0, -22.0, -2.0];
        let point = intersection(&a, &b);

        assert!(dbg!(point).is_none());
    }
}
