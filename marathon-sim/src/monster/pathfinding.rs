use std::collections::VecDeque;

use glam::Vec2;

/// Find the shortest path through the polygon adjacency graph from
/// `start_polygon` to `target_polygon` using BFS.
///
/// Returns the sequence of polygon indices to traverse (including start and target),
/// or None if no path exists.
pub fn find_polygon_path(
    start_polygon: usize,
    target_polygon: usize,
    polygon_adjacency: &[Vec<(usize, Option<usize>)>],
    polygon_count: usize,
) -> Option<Vec<usize>> {
    if start_polygon == target_polygon {
        return Some(vec![start_polygon]);
    }

    let mut visited = vec![false; polygon_count];
    let mut parent = vec![usize::MAX; polygon_count];
    let mut queue = VecDeque::new();

    visited[start_polygon] = true;
    queue.push_back(start_polygon);

    while let Some(current) = queue.pop_front() {
        for &(_line_idx, adj) in &polygon_adjacency[current] {
            if let Some(adj_idx) = adj {
                if !visited[adj_idx] {
                    visited[adj_idx] = true;
                    parent[adj_idx] = current;
                    if adj_idx == target_polygon {
                        // Reconstruct path
                        return Some(reconstruct_path(&parent, start_polygon, target_polygon));
                    }
                    queue.push_back(adj_idx);
                }
            }
        }
    }

    None // No path found
}

fn reconstruct_path(parent: &[usize], start: usize, target: usize) -> Vec<usize> {
    let mut path = Vec::new();
    let mut current = target;
    while current != start {
        path.push(current);
        current = parent[current];
    }
    path.push(start);
    path.reverse();
    path
}

/// Get the center point of a polygon (average of vertices).
pub fn polygon_center(vertices: &[Vec2]) -> Vec2 {
    if vertices.is_empty() {
        return Vec2::ZERO;
    }
    let sum: Vec2 = vertices.iter().copied().sum();
    sum / vertices.len() as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_same_polygon() {
        let adjacency = vec![vec![]];
        let path = find_polygon_path(0, 0, &adjacency, 1);
        assert_eq!(path, Some(vec![0]));
    }

    #[test]
    fn path_adjacent_polygons() {
        // 0 <-> 1
        let adjacency = vec![
            vec![(0, Some(1))],
            vec![(0, Some(0))],
        ];
        let path = find_polygon_path(0, 1, &adjacency, 2);
        assert_eq!(path, Some(vec![0, 1]));
    }

    #[test]
    fn path_through_chain() {
        // 0 <-> 1 <-> 2
        let adjacency = vec![
            vec![(0, Some(1))],
            vec![(0, Some(0)), (1, Some(2))],
            vec![(1, Some(1))],
        ];
        let path = find_polygon_path(0, 2, &adjacency, 3);
        assert_eq!(path, Some(vec![0, 1, 2]));
    }

    #[test]
    fn path_no_connection() {
        // 0 and 1 not connected
        let adjacency = vec![
            vec![],
            vec![],
        ];
        let path = find_polygon_path(0, 1, &adjacency, 2);
        assert_eq!(path, None);
    }

    #[test]
    fn polygon_center_calculation() {
        let vertices = vec![
            Vec2::new(0.0, 0.0),
            Vec2::new(2.0, 0.0),
            Vec2::new(2.0, 2.0),
            Vec2::new(0.0, 2.0),
        ];
        let center = polygon_center(&vertices);
        assert!((center.x - 1.0).abs() < f32::EPSILON);
        assert!((center.y - 1.0).abs() < f32::EPSILON);
    }
}
