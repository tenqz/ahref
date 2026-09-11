//! Sparse power iteration with uniform dangling mass and L1 convergence.
pub(super) fn pagerank(adjacency: &[Vec<usize>], damping: f64) -> (Vec<f64>, usize, bool) {
    let n = adjacency.len();
    if n == 0 {
        return (Vec::new(), 0, true);
    }
    let mut ranks = vec![1.0 / n as f64; n];
    for iteration in 1..=1000 {
        let dangling: f64 = adjacency
            .iter()
            .zip(&ranks)
            .filter(|(links, _)| links.is_empty())
            .map(|(_, r)| r)
            .sum();
        let mut next = vec![(1.0 - damping + damping * dangling) / n as f64; n];
        for (a, links) in adjacency.iter().enumerate() {
            for &b in links {
                next[b] += damping * ranks[a] / links.len() as f64;
            }
        }
        let delta: f64 = ranks.iter().zip(&next).map(|(a, b)| (a - b).abs()).sum();
        ranks = next;
        if delta < 1e-12 {
            return (ranks, iteration, true);
        }
    }
    (ranks, 1000, false)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rank_cycle_and_dangling() {
        let (r, _, ok) = pagerank(&[vec![1], vec![2], vec![0]], 0.85);
        assert!(ok);
        assert!(r.iter().all(|x| (x - 1.0 / 3.0).abs() < 1e-10));
        let (r, _, ok) = pagerank(&[vec![1], vec![], vec![1]], 0.85);
        assert!(ok);
        assert!((r.iter().sum::<f64>() - 1.0).abs() < 1e-10);
        assert!(r[1] > r[0]);
        assert!(pagerank(&[], 0.85).0.is_empty());
    }
}
