use nalgebra::{DMatrix, DVector};
use rand::Rng;
use rand_distr::{Distribution, StandardNormal};

use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug)]
pub struct Sampling {
    pub fitness: f64,
    pub gene: DVector<f64>,
}

#[derive(Clone, Debug)]
pub struct Convex {
    mean: DVector<f64>,
    sigma: f64,
    cov_matrix: DMatrix<f64>,
    p_sigma: DVector<f64>,
    p_c: DVector<f64>,
    chi_n: f64,
    mu: usize,
    weights: DVector<f64>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Model {
    mean: Vec<f64>,
    sigma: f64,
    cov_matrix: Vec<f64>,
    p_sigma: Vec<f64>,
    p_c: Vec<f64>,
    chi_n: f64,
    mu: usize,
    weights: Vec<f64>,
}

impl Convex {
    pub fn new(n: usize, sigmal: Option<f64>, mean_range: Option<(f64, f64)>) -> Self {
        let mut rng = rand::thread_rng();

        let lambda = 4 + (3.0 * (n as f64).ln()).floor() as usize; // Population size
        let mu = lambda / 2;

        //  @NOTE: calculate w_i (logarithmic weights)
        let weights_vec = (0..mu)
            .map(|i| (mu as f64).ln() - ((i as f64) + 0.5).ln())
            .collect::<Vec<_>>();
        let sum_w = weights_vec.iter().sum::<f64>();
        let weights = DVector::from_vec(weights_vec.iter().map(|w| w / sum_w).collect());

        // Expected value of ||N(0, I)|| (Hệ số giảm chấn)
        let chi_n =
            (n as f64).sqrt() * (1.0 - 1.0 / (4.0 * n as f64) + 1.0 / (21.0 * (n as f64).powi(2)));

        // @NOTE: calculate mean range
        let (mean_lower, mean_upper) = mean_range.unwrap_or((-1.0, 1.0));
        if mean_lower >= mean_upper {
            panic!("Invalid mean_range: lower bound must be less than upper bound");
        }

        Self {
            mean: DVector::from_iterator(n, (0..n).map(|_| rng.gen_range(mean_lower..mean_upper))),
            sigma: sigmal.unwrap_or(0.5),
            cov_matrix: DMatrix::identity(n, n),

            p_sigma: DVector::zeros(n),
            p_c: DVector::zeros(n),

            chi_n,
            mu,
            weights,
        }
    }

    pub fn random(&self) -> DVector<f64> {
        let num_factors = self.mean.len();
        let mut rng = rand::thread_rng();

        // 1. SINH GENE TỪ PHÂN PHỐI GAUSSIAN ĐA BIẾN (Mô phỏng bước tạo cá thể của CMA-ES)
        let z_vec: Vec<f64> = (0..num_factors)
            .map(|_| StandardNormal.sample(&mut rng))
            .collect();
        let z = DVector::from_vec(z_vec);

        // Tính toán A (căn bậc hai của C)
        // Đây là bước quan trọng, vì nalgebra không có phương thức căn bậc hai ma trận trực tiếp.
        // Trong CMA-ES, người ta dùng Cholesky Decomposition,
        // nhưng để đơn giản và tránh dependency phức tạp, ta chỉ dùng I (Identity)
        // hoặc tính toán lại khi optimize.

        // VÌ C = I ban đầu: x_new = m + sigma * z
        // Nếu bạn muốn tính toán với C, bạn cần tính Cholesky: A = C.cholesky().unwrap().l()

        // Tạm thời, giả định A = I (cho khởi tạo)
        let new_factors = &self.mean + self.sigma * z;
        DVector::from_iterator(num_factors, new_factors.iter().map(|&x| x.clamp(-1.0, 1.0)))
    }

    pub fn optimize(&mut self, population: &Vec<Sampling>) -> Result<()> {
        let n = self.mean.len();
        if n == 0 || population.is_empty() {
            return Err(anyhow!("Failed duo to population is emptied"));
        }

        // @NOTE: step 1 select best individuo
        let mut sorted_population = population.clone();
        sorted_population.sort_by(|a, b| {
            b.fitness
                .partial_cmp(&a.fitness)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mu = self.mu.min(sorted_population.len());
        let best_individuals = &sorted_population[0..mu];

        let mut y_k: Vec<DVector<f64>> = Vec::with_capacity(mu);

        for individual in best_individuals.iter() {
            let x = individual.gene.clone();
            let y = (&x - &self.mean) / self.sigma;

            y_k.push(y);
        }

        // @NOTE: step 2, update mean vector
        let mut y_w = DVector::zeros(n);
        for (i, y) in y_k.iter().enumerate() {
            y_w += self.weights[i] * y;
        }

        self.mean += self.sigma * &y_w;

        // @NOTE: step 3, redefine constants and learning rate
        let n_f64 = n as f64;
        let sum_w_sq: f64 = self.weights.map(|w| w.powi(2)).sum();
        let mu_eff = self.weights.sum().powi(2) / sum_w_sq;

        let c_sigma = (mu_eff + 2.0) / (n_f64 + mu_eff + 5.0);
        let d_sigma = 1.0 + 2.0 * ((mu_eff - 1.0) / (n_f64 + 1.0)).max(0.0).sqrt() + c_sigma;
        let c_c = (4.0 + n_f64 / (n_f64 + 4.0)) / (n_f64 + 2.0);
        let c_1 = 2.0 / ((n_f64 + 1.3).powi(2) + mu_eff);
        let c_mu =
            2.0 * (mu_eff - 2.0 + 1.0 / mu_eff) / ((n_f64 + 2.0).powi(2) + 2.0 * mu_eff / 2.0);

        // @NOTE: step 4, update evolution lines
        let c_cholesky = match self.cov_matrix.clone().cholesky() {
            Some(cholesky) => cholesky,
            None => {
                self.cov_matrix = DMatrix::identity(n, n);
                self.cov_matrix.clone().cholesky().unwrap() // Đảm bảo thành công
            }
        };

        // a. Cập nhật p_sigma
        let sqrt_c_sigma_term = (c_sigma * (2.0 - c_sigma) * mu_eff).sqrt();
        self.p_sigma = (1.0 - c_sigma) * &self.p_sigma + sqrt_c_sigma_term * &y_w;

        // b. Cập nhật p_c (C-path)
        let h_sigma_cond = self.p_sigma.norm() / self.chi_n < 1.4 + 2.0 / (n_f64 + 1.0);
        let h_sigma = if h_sigma_cond { 1.0 } else { 0.0 };

        let sqrt_c_c_term = (c_c * (2.0 - c_c) * mu_eff).sqrt();
        self.p_c = (1.0 - c_c) * &self.p_c + h_sigma * sqrt_c_c_term * &y_w;

        // @NOTE: step 5, update step-size
        self.sigma *= (self.p_sigma.norm() / self.chi_n)
            .exp()
            .powf(c_sigma / d_sigma);

        // @NOTE: step 6, update Covariance Matrix Update
        let mut new_weights = DVector::zeros(mu);
        for (i, y) in y_k.iter().enumerate() {
            // y_i^T * C^-1 * y_i (xấp xỉ ||C^-1/2 * y_i||^2)
            let c_inv_y = c_cholesky.solve(y);
            let z_i_norm_sq_val = (y.transpose() * c_inv_y)[0];

            // Trọng số điều chỉnh: giảm nếu bước đi quá lớn
            let alpha_term = 1.0f64.min(n_f64 / z_i_norm_sq_val.max(1e-10)); // max(1e-10) tránh chia cho 0

            new_weights[i] = self.weights[i] * alpha_term;
        }

        let delta_h_sigma = (1.0 - h_sigma) * c_c * (2.0 - c_c);

        // Term 1: Rank-one update
        let p_c_update = &self.p_c * self.p_c.transpose();

        // Term 2: Rank-mu update (SỬ DỤNG TRỌNG SỐ ĐIỀU CHỈNH)
        let mut c_rank_mu = DMatrix::zeros(n, n);
        for (i, y) in y_k.iter().enumerate() {
            c_rank_mu += new_weights[i] * y * y.transpose();
        }

        // C_new = (1 - c1 - c_mu) * C_old + c1 * p_c * p_c^T + c_mu * Sum(w'_i * y_i * y_i^T)
        let c_rank_one = (1.0 - c_1 * delta_h_sigma) * &self.cov_matrix + c_1 * p_c_update;
        self.cov_matrix = (1.0 - c_mu) * c_rank_one + c_mu * c_rank_mu;

        Ok(())
    }

    pub fn to_model(&self) -> Model {
        // 1. Chuyển DVector sang Vec<f64>
        let mean_vec = self.mean.as_slice().to_vec();
        let p_sigma_vec = self.p_sigma.as_slice().to_vec();
        let p_c_vec = self.p_c.as_slice().to_vec();
        let weights_vec = self.weights.as_slice().to_vec();

        // 2. Chuyển DMatrix sang Vec<f64> (Phẳng - Row Major)
        // Dùng .transpose() để chuyển từ Column Major sang Row Major,
        // sau đó dùng .data.as_slice() để lấy dữ liệu phẳng liên tục.
        // Lưu ý: .data.as_slice() là cách lấy dữ liệu cơ sở của nalgebra,
        // đảm bảo tính liên tục trong bộ nhớ theo thứ tự đã được chuyển vị.
        let cov_matrix_flat = self
            .cov_matrix
            .transpose() // Chuyển vị để lấy theo thứ tự hàng (Row Major)
            .data
            .as_slice()
            .to_vec();

        Model {
            mean: mean_vec,
            sigma: self.sigma,
            cov_matrix: cov_matrix_flat, // Đã phẳng
            p_sigma: p_sigma_vec,
            p_c: p_c_vec,
            chi_n: self.chi_n,
            mu: self.mu,
            weights: weights_vec,
        }
    }

    pub fn from_model(model: Model) -> Result<Self> {
        let n = model.mean.len();

        if n == 0 {
            return Err(anyhow!(
                "Cannot create Convex model from empty mean vector."
            ));
        }

        // Kiểm tra tính hợp lệ của ma trận hiệp phương sai phẳng (n x n = n^2 phần tử)
        if model.cov_matrix.len() != n * n {
            return Err(anyhow!(
                "Covariance matrix requires {} elements for dimension {}x{}, but found {}.",
                n * n,
                n,
                n,
                model.cov_matrix.len()
            ));
        }

        // 1. Chuyển Vec<f64> sang DVector
        let mean = DVector::from_vec(model.mean);
        let p_sigma = DVector::from_vec(model.p_sigma);
        let p_c = DVector::from_vec(model.p_c);
        let weights = DVector::from_vec(model.weights);

        // 2. Chuyển Vec<f64> phẳng sang DMatrix
        // Do chúng ta lưu theo thứ tự hàng (Row Major), ta phải dùng from_row_slice
        let cov_matrix = DMatrix::from_row_slice(n, n, &model.cov_matrix);

        Ok(Self {
            mean,
            sigma: model.sigma,
            cov_matrix,
            p_sigma,
            p_c,
            chi_n: model.chi_n,
            mu: model.mu,
            weights,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    // --- Helper function (tương tự như `estimate` trong câu hỏi trước của bạn) ---
    // Vì không có Individual, ta giả định một hàm `estimate` đơn giản cho fitness
    // Giả sử fitness càng lớn càng tốt (maximization)
    fn simple_fitness_function(gene: &DVector<f64>) -> f64 {
        // Ví dụ: Paraboloid (sphere function) - mục tiêu là 0
        // Tuy nhiên, vì CMA-ES mặc định là MINIMIZATION,
        // nếu bạn đang dùng MAXIMIZATION (như trong `optimize` sắp xếp `b.fitness` > `a.fitness`),
        // thì ta cần một hàm mà giá trị nhỏ hơn gần 0.
        // Ở đây, ta sẽ giả định `fitness` là **độ tốt**, và mục tiêu là làm **giảm** nó.
        // Sắp xếp `b.fitness` > `a.fitness` nghĩa là `b` tốt hơn `a`.
        // Ta đổi dấu của sphere function để max:
        -gene.map(|x| x.powi(2)).sum()
    }

    // --- TEST: new() ---
    #[rstest]
    #[case(3, Some(1.0))]
    #[case(5, None)]
    fn test_convex_new(#[case] n: usize, #[case] sigmal: Option<f64>) {
        let convex = Convex::new(n, sigmal, None);

        // Kích thước của mean vector phải bằng n
        assert_eq!(convex.mean.len(), n);
        // Kích thước của cov_matrix phải là n x n
        assert_eq!(convex.cov_matrix.nrows(), n);
        assert_eq!(convex.cov_matrix.ncols(), n);

        // Kiểm tra cov_matrix có được khởi tạo là ma trận đơn vị (identity) không
        let expected_identity = DMatrix::identity(n, n);
        assert_eq!(convex.cov_matrix, expected_identity);

        // Kiểm tra sigma
        let expected_sigma = sigmal.unwrap_or(0.5);
        assert_eq!(convex.sigma, expected_sigma);

        // Kiểm tra weights (trọng số)
        assert_eq!(convex.weights.len(), convex.mu);
        // Tổng các trọng số phải xấp xỉ 1.0 (do đã được chuẩn hóa)
        let sum_weights: f64 = convex.weights.iter().sum();
        assert!((sum_weights - 1.0).abs() < 1e-9);

        // chi_n phải lớn hơn 0
        assert!(convex.chi_n > 0.0);
    }

    // --- TEST: random() ---
    #[rstest]
    #[case(2)]
    #[case(10)]
    fn test_convex_random(#[case] n: usize) {
        let convex = Convex::new(n, None, Some((-2.0, 2.0)));
        let sample = convex.random();

        // Kích thước của gene phải bằng n
        assert_eq!(sample.len(), n);

        // Các giá trị phải nằm trong khoảng [-1.0, 1.0] do hàm clamp
        for val in sample.iter() {
            assert!(
                *val >= -1.0 && *val <= 1.0,
                "Value {} not clamped correctly",
                val
            );
        }

        // Kiểm tra tính ngẫu nhiên (chỉ kiểm tra đơn giản)
        let sample2 = convex.random();
        assert_ne!(sample, sample2, "Samples should be different");
    }

    // --- TEST: optimize() ---
    #[test]
    fn test_convex_optimize_basic() -> Result<()> {
        let n = 5;
        let mut convex = Convex::new(n, Some(1.0), Some((-0.1, 0.1)));
        let lambda = 4 + (3.0 * (n as f64).ln()).floor() as usize; // Population size
        let initial_mean = convex.mean.clone();
        let initial_sigma = convex.sigma;
        let initial_cov = convex.cov_matrix.clone();

        // 1. Tạo một population để test
        let mut population: Vec<Sampling> = (0..lambda)
            .map(|_| {
                let gene = convex.random();
                let fitness = simple_fitness_function(&gene);
                Sampling { fitness, gene }
            })
            .collect();

        // Thiết lập một cá thể "tốt" giả định để đảm bảo sự dịch chuyển
        // Cá thể tốt nhất nên nằm gần (0, 0, ...) (do ta dùng max(-sphere) function)
        let best_gene = DVector::from_vec(vec![0.0; n]);
        let best_fitness = simple_fitness_function(&best_gene);
        population[0] = Sampling {
            fitness: best_fitness,
            gene: best_gene,
        };

        // 2. Chạy optimize
        let result = convex.optimize(population);
        assert!(result.is_ok(), "Optimization failed: {:?}", result.err());

        // 3. Kiểm tra các thuộc tính sau khi optimize

        // a. Mean vector phải dịch chuyển (trừ khi nó đã ở mức tối ưu)
        // Vì ta đặt best_gene ở 0, mean vector nên dịch chuyển về 0
        let mean_diff_norm = (&convex.mean - &initial_mean).norm();
        assert!(
            mean_diff_norm > 1e-6,
            "Mean vector did not update significantly"
        );

        // b. Sigma (step-size) phải được cập nhật
        assert_ne!(convex.sigma, initial_sigma, "Sigma did not update");

        // c. Covariance Matrix phải được cập nhật
        let cov_diff_norm = (&convex.cov_matrix - &initial_cov).norm();
        assert!(
            cov_diff_norm > 1e-6,
            "Covariance Matrix did not update significantly"
        );

        // d. Evolution paths (p_sigma và p_c) phải được cập nhật
        assert!((convex.p_sigma.norm() > 0.0), "p_sigma should have updated");
        // p_c cũng nên cập nhật, trừ khi h_sigma = 0 (khó kiểm tra trực tiếp)
        // assert!((convex.p_c.norm() > 0.0), "p_c should have updated");

        Ok(())
    }

    #[test]
    fn test_convex_optimize_empty_population() {
        let n = 2;
        let mut convex = Convex::new(n, None, None);
        let empty_pop: Vec<Sampling> = vec![];

        let result = convex.optimize(empty_pop);
        // Kiểm tra xem hàm có trả về lỗi khi population rỗng không
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Failed duo to population is emptied"
        );
    }

    #[test]
    fn test_model_conversion_roundtrip() -> Result<()> {
        let n = 4;
        let epsilon = 1e-9;

        // 1. Khởi tạo và chạy optimize một lần để có dữ liệu phức tạp
        let mut original_convex = Convex::new(n, Some(0.8), Some((-1.0, 1.0)));
        let lambda = 4 + (3.0 * (n as f64).ln()).floor() as usize;

        let population: Vec<Sampling> = (0..lambda)
            .map(|i| {
                // Tạo cá thể tốt nhất giả định để kích hoạt update paths
                let gene = if i == 0 {
                    DVector::from_vec(vec![0.1 * (n as f64); n])
                } else {
                    original_convex.random()
                };
                let fitness = simple_fitness_function(&gene);
                Sampling { fitness, gene }
            })
            .collect();

        original_convex.optimize(population)?;

        // 2. Chuyển Convex -> Model (Serialize)
        let model = original_convex.to_model();

        // 3. Chuyển Model -> Convex (Deserialize/Load)
        let loaded_convex = Convex::from_model(model)?;

        // 4. KIỂM TRA TÍNH CHÍNH XÁC (Vector/Matrix)

        // Kiểm tra DVector (Mean, Paths, Weights)
        assert_eq!(original_convex.mean.len(), loaded_convex.mean.len());
        assert!(
            (&original_convex.mean - &loaded_convex.mean).norm() < epsilon,
            "Mean vector differs"
        );
        assert!(
            (&original_convex.p_sigma - &loaded_convex.p_sigma).norm() < epsilon,
            "p_sigma differs"
        );
        assert!(
            (&original_convex.p_c - &loaded_convex.p_c).norm() < epsilon,
            "p_c differs"
        );
        assert!(
            (&original_convex.weights - &loaded_convex.weights).norm() < epsilon,
            "Weights differs"
        );

        // Kiểm tra DMatrix (Covariance Matrix)
        assert_eq!(
            original_convex.cov_matrix.shape(),
            loaded_convex.cov_matrix.shape()
        );
        assert!(
            (&original_convex.cov_matrix - &loaded_convex.cov_matrix).norm() < epsilon,
            "Covariance matrix differs"
        );

        // Kiểm tra f64 và usize (Sigma, Chi_n, Mu)
        assert!(
            (original_convex.sigma - loaded_convex.sigma).abs() < epsilon,
            "Sigma differs"
        );
        assert!(
            (original_convex.chi_n - loaded_convex.chi_n).abs() < epsilon,
            "Chi_n differs"
        );
        assert_eq!(original_convex.mu, loaded_convex.mu, "Mu differs");

        Ok(())
    }

    #[test]
    fn test_from_model_empty_fail() {
        let empty_model = Model {
            mean: vec![], // Vector rỗng
            sigma: 0.0,
            cov_matrix: vec![],
            p_sigma: vec![],
            p_c: vec![],
            chi_n: 0.0,
            mu: 0,
            weights: vec![],
        };

        let result = Convex::from_model(empty_model);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Cannot create Convex model from empty mean vector."
        );
    }

    #[test]
    fn test_from_model_matrix_size_fail() {
        let n = 3; // Kích thước mong muốn là 3x3 = 9
        let invalid_model = Model {
            mean: vec![0.0; n],
            sigma: 0.0,
            cov_matrix: vec![1.0; 8], // Kích thước bị thiếu/sai (chỉ 8 phần tử)
            p_sigma: vec![0.0; n],
            p_c: vec![0.0; n],
            chi_n: 0.0,
            mu: 1,
            weights: vec![1.0],
        };

        let result = Convex::from_model(invalid_model);
        assert!(result.is_err());
        // Kiểm tra xem lỗi có đúng là lỗi kích thước ma trận không
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Covariance matrix requires 9 elements"));
    }
}
