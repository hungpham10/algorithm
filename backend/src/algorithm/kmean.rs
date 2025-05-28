use rayon::prelude::*;

use std::fmt::Debug;
use std::marker::{Send, Sync};
use std::sync::Arc;

use rand::Rng;

#[derive(Debug, Clone)]
struct Point<T> {
    point: Arc<T>,
}

impl<T: Clone + Debug + Send + Sync> Point<T> {
    fn new(point: Arc<T>) -> Self {
        Point { point }
    }
}

type DistantCallback<T> = fn(point: Arc<T>, cluster: &Vec<f64>) -> f64;

pub trait Strategy<T> {
    // @NOTE: configure clusters
    fn initialize(&mut self, points: &[Arc<T>]) -> Vec<Vec<usize>>;
    fn optimize(&mut self, cluster: usize, points: &[usize]) -> Vec<f64>;
    fn parameters(&self, cluster: usize) -> Vec<f64>;

    // @NOTE: collect callbacks
    fn get_distant_fn(&self) -> DistantCallback<T>;
}

#[derive(Debug)]
pub struct KMean<T, S: Strategy<T>> {
    points: Vec<Point<T>>,
    clusters: Vec<Vec<usize>>,
    distants: Vec<f64>,
    strategy: S,
    max_iteration: usize,
}

impl<T: Clone + Debug + Send + Sync, S: Strategy<T>> KMean<T, S> {
    pub fn new(number_of_cluster: usize, max_iteration: usize, strategy: S) -> Self {
        let points = Vec::<Point<T>>::new();
        let distants = Vec::<f64>::new();
        let clusters = vec![Vec::<usize>::new(); number_of_cluster];

        KMean {
            points,
            clusters,
            max_iteration,
            strategy,
            distants,
        }
    }

    pub fn cluster(&self, id: usize) -> Vec<f64> {
        self.strategy.parameters(id)
    }

    pub fn points(&self, cluster: usize) -> Vec<Arc<T>> {
        self.clusters[cluster]
            .iter()
            .map(|i| self.points[*i].point.clone())
            .collect::<Vec<Arc<T>>>()
    }

    pub fn insert(&mut self, points: &[T]) {
        points.iter().for_each(|point| {
            self.points.push(Point::new(Arc::new(point.clone())));
            self.distants.push(f64::MAX);
        });
    }

    pub fn commit(&mut self) {
        let points = self
            .points
            .iter()
            .map(|point| point.point.clone())
            .collect::<Vec<Arc<T>>>();

        self.clusters = self.strategy.initialize(&points);

        (0..self.clusters.len()).for_each(|i| {
            self.strategy.optimize(i, &self.clusters[i]);
        });
    }

    pub fn fit(&mut self) -> f64 {
        let mut parameters = self
            .clusters
            .iter()
            .enumerate()
            .map(|(i, _)| self.strategy.parameters(i))
            .collect::<Vec<Vec<f64>>>();
        let distant_fn = self.strategy.get_distant_fn();

        for _ in 0..self.max_iteration {
            self.clusters.iter_mut().for_each(|cluster| {
                cluster.clear();
            });

            // @NOTE: calculate distance between points and clusters in parallel
            let distributed = self
                .points
                .par_iter()
                .enumerate()
                .map(|(ipoint, point)| {
                    let (cluster, distance) = parameters
                        .par_iter()
                        .enumerate()
                        .map(|(icluster, cluster)| {
                            (icluster, (distant_fn)(point.point.clone(), cluster))
                        })
                        .reduce(|| (0, f64::MAX), |a, b| if a.1 < b.1 { a } else { b });

                    (cluster, ipoint, distance)
                })
                .collect::<Vec<(usize, usize, f64)>>();

            // @NOTE: assign points to clusters in parallel
            for (cluster, ipoint, distance) in distributed {
                self.clusters[cluster].push(ipoint);
                self.distants[ipoint] = distance;
            }

            // @NOTE: update centroids of each cluster
            parameters = (0..parameters.len())
                .map(|i| self.strategy.optimize(i, &self.clusters[i]))
                .collect::<Vec<Vec<f64>>>();
        }

        self.distants.iter().sum::<f64>() / self.distants.len() as f64
    }
}

pub struct LineStrategy {
    lines: Vec<(f64, f64)>,
    points: Vec<(f64, f64)>,
    slope_range: (f64, f64),
    intecept_range: (f64, f64),
}

impl LineStrategy {
    pub fn new(k: usize, slope_range: (f64, f64), intecept_range: (f64, f64)) -> Self {
        LineStrategy {
            lines: vec![(0.0, 0.0); k],
            points: vec![],
            slope_range,
            intecept_range,
        }
    }
}

impl Strategy<(f64, f64)> for LineStrategy {
    fn initialize(&mut self, points: &[Arc<(f64, f64)>]) -> Vec<Vec<usize>> {
        let k = self.lines.len();
        let mut rng = rand::thread_rng();
        let mut parts = vec![0; k];
        let mut clusters = vec![Vec::new(); k];

        for i in 0..(k - 1) {
            if i == 0 {
                parts[i] = rng.gen_range(points.len() / (2 * k)..points.len() / k);
            } else {
                parts[i] = rng.gen_range(
                    (parts[i - 1] + points.len() / (2 * k))..(parts[i - 1] + points.len() / k),
                );
            }
        }

        parts[k - 1] = points.len();

        for i in 0..points.len() {
            for (j, p) in parts.iter().enumerate() {
                if *p > i {
                    clusters[j].push(i);
                    break;
                }
            }
        }

        for i in 0..k {
            self.lines[i] = (
                rng.gen_range(self.slope_range.0..self.slope_range.1),
                rng.gen_range(self.intecept_range.0..self.intecept_range.1),
            );
        }

        self.points.clear();

        points
            .iter()
            .for_each(|point| self.points.push((point.0, point.1)));

        clusters
    }

    fn optimize(&mut self, cluster: usize, points: &[usize]) -> Vec<f64> {
        if points.len() < 2 {
            return vec![self.lines[cluster].0, self.lines[cluster].1];
        }

        let n = points.len() as f64;

        let mean_x = points.iter().map(|i| self.points[*i].0).sum::<f64>() / n;
        let mean_y = points.iter().map(|i| self.points[*i].1).sum::<f64>() / n;

        let mut variance_x = 0.0;
        let mut covariance = 0.0;

        for i in points {
            let diff_x = self.points[*i].0 - mean_x;
            let diff_y = self.points[*i].1 - mean_y;

            variance_x = diff_x.powi(2);
            covariance = diff_y * diff_x;
        }

        if variance_x.abs() >= f64::EPSILON {
            let slope = covariance / variance_x;
            let intercept = mean_y - slope * mean_x;

            self.lines[cluster] = (slope, intercept);
        }

        vec![self.lines[cluster].0, self.lines[cluster].1]
    }

    fn parameters(&self, cluster: usize) -> Vec<f64> {
        vec![self.lines[cluster].0, self.lines[cluster].1]
    }

    fn get_distant_fn(&self) -> DistantCallback<(f64, f64)> {
        |point, cluster| {
            // Formula: |mx - y + b| / √(m² + 1)
            let x = point.0;
            let y = point.1;
            let slope = cluster[0];
            let intercept = cluster[1];

            (slope * x + intercept - y).abs() / (slope.powi(2) + 1.0).sqrt()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    use rand::Rng;

    #[derive(Debug, Clone)]
    struct TestPoint {
        x: f64,
        y: f64,
    }

    #[derive(Debug)]
    struct TestStrategy {
        points: Vec<TestPoint>,
        centroids: Vec<TestPoint>,
    }

    impl Strategy<TestPoint> for TestStrategy {
        fn initialize(&mut self, points: &Vec<Arc<TestPoint>>) -> Vec<Vec<usize>> {
            // Assign points to initial clusters randomly
            let mut rng = rand::thread_rng();
            let k = 3; // Number of clusters
            let mut clusters = vec![Vec::new(); k];

            for i in 0..points.len() {
                let cluster_index = rng.gen_range(0..k);
                clusters[cluster_index].push(i);
            }

            for i in 0..k {
                self.centroids.push(TestPoint {
                    x: rng.gen_range(0.0..10.0),
                    y: rng.gen_range(0.0..10.0),
                });
            }

            self.points.clear();
            points.iter().for_each(|point| {
                self.points.push(TestPoint {
                    x: point.x,
                    y: point.y,
                })
            });
            clusters
        }

        fn optimize(&mut self, cluster: usize, points: &[usize]) -> Vec<f64> {
            if points.is_empty() {
                return vec![0.0, 0.0]; // Handle empty cluster case
            }

            let mut sum_x = 0.0;
            let mut sum_y = 0.0;
            for &point_index in points {
                sum_x += self.points[point_index].x;
                sum_y += self.points[point_index].y;
            }

            self.centroids[cluster].x = sum_x / points.len() as f64;
            self.centroids[cluster].y = sum_y / points.len() as f64;

            vec![self.centroids[cluster].x, self.centroids[cluster].y]
        }

        fn parameters(&self, cluster: usize) -> Vec<f64> {
            vec![self.centroids[cluster].x, self.centroids[cluster].y]
        }

        fn get_distant_fn(&self) -> DistantCallback<TestPoint> {
            |point: Arc<TestPoint>, cluster: &Vec<f64>| -> f64 {
                let dx = point.x - cluster[0];
                let dy = point.y - cluster[1];
                (dx * dx + dy * dy).sqrt()
            }
        }
    }

    #[test]
    fn test_kmean_with_strategy() {
        let mut kmean = KMean::new(
            3,  // Number of clusters
            10, // Max iterations
            TestStrategy {
                points: Vec::new(),
                centroids: Vec::new(),
            },
        );

        let points = vec![
            TestPoint { x: 1.0, y: 1.0 },
            TestPoint { x: 1.5, y: 2.0 },
            TestPoint { x: 3.0, y: 4.0 },
            TestPoint { x: 5.0, y: 7.0 },
            TestPoint { x: 3.5, y: 5.0 },
            TestPoint { x: 4.5, y: 5.0 },
            TestPoint { x: 3.5, y: 4.5 },
        ];

        kmean.insert(&points);
        kmean.commit();

        // Fit the KMeans model
        kmean.fit();

        // Assertions could be added here based on the expected clustering behavior
        // given the test data and strategy.  Since the initialization is random,
        // it's difficult to assert on specific cluster assignments.  Instead,
        // you might assert on properties like the WCSS decreasing with iterations
        // (if you modify fit to return WCSS per iteration), or that all points
        // are assigned to a cluster.

        // Simple check to ensure all points are in a cluster:
        let total_points_in_clusters: usize = kmean.clusters.iter().map(|c| c.len()).sum();
        assert_eq!(total_points_in_clusters, points.len());
    }

    #[test]
    fn test_kmean_with_line_strategy() {
        let mut kmean = KMean::new(
            2,  // Number of clusters
            10, // Max iterations
            LineStrategy::new(2, (-10.0, 10.0), (-10.0, 10.0)),
        );

        let points = vec![
            (1.0, 1.0),
            (1.5, 2.0),
            (3.0, 4.0),
            (5.0, 7.0),
            (3.5, 5.0),
            (4.5, 5.0),
            (3.5, 4.5),
        ];

        kmean.insert(&points);
        kmean.commit();

        // Fit the KMeans model
        kmean.fit();

        println!("cluster {:?}", kmean.cluster(0));
        println!("{:?}", kmean.points(0));
        println!("cluster {:?}", kmean.cluster(1));
        println!("{:?}", kmean.points(1));
    }
}
