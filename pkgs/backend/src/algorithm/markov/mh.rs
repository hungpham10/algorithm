use log::{debug, error};
use rand::prelude::*;
use rand_distr::Normal;

pub type LogLikelihoodCallback = fn(data: &[f64], mu: f64, sigma: f64) -> f64;
pub type LogPriorCallback = fn(f64, f64) -> f64;

struct MetropolisHasting {
    log_likelihood: LogLikelihoodCallback,
    log_prior: LogPriorCallback,
    number_of_epochs: usize,
}

impl MetropolisHasting {
    pub fn new(
        number_of_epochs: usize,
        log_prior: LogPriorCallback,
        log_likelihood: LogLikelihoodCallback,
    ) -> Self {
        Self {
            number_of_epochs,
            log_prior,
            log_likelihood,
        }
    }

    pub fn metropolis_hastings(
        &self,
        data: &[f64],
        mu_initial: f64,
        sigma_initial: f64,
        seed: u64,
    ) -> Vec<(f64, f64)> {
        let mut ret = Vec::with_capacity(self.number_of_epochs);
        let mut rng = StdRng::seed_from_u64(seed);
        let mut mu_current = mu_initial;
        let mut sigma_current = sigma_initial.max(0.01);
        let mut log_posterior_current = self.log_posterior(data, mu_current, sigma_current);
        let mut acceptances = 0;

        for i in 0..self.number_of_epochs {
            let (mu_std_dev, sigma_std_dev) =
                self.calculate_proposal_std_dev(data, mu_current, sigma_current);
            let mu_proposal = match Normal::new(mu_current, mu_std_dev) {
                Ok(normal) => rng.sample(normal),
                Err(_) => {
                    error!(
                        "Invalid mu proposal at iteration {}: std_dev={}",
                        i, mu_std_dev
                    );
                    mu_current
                }
            };
            let sigma_proposal = match Normal::new(sigma_current, sigma_std_dev) {
                Ok(normal) => rng.sample(normal).max(0.01),
                Err(_) => {
                    error!(
                        "Invalid sigma proposal at iteration {}: std_dev={}",
                        i, sigma_std_dev
                    );
                    sigma_current
                }
            };
            let log_posterior_proposal = self.log_posterior(data, mu_proposal, sigma_proposal);

            let acceptance_ratio =
                if log_posterior_proposal.is_finite() && log_posterior_current.is_finite() {
                    (log_posterior_proposal - log_posterior_current)
                        .exp()
                        .min(1.0)
                } else {
                    0.0
                };

            if rng.gen::<f64>() < acceptance_ratio {
                mu_current = mu_proposal;
                sigma_current = sigma_proposal;
                log_posterior_current = log_posterior_proposal;
                acceptances += 1;
            }

            ret.push((mu_current, sigma_current));
            if cfg!(debug_assertions) && i % 1000 == 0 {
                debug!(
                    "Iteration {}: mu={}, sigma={}, posterior={}, acceptance_rate={}",
                    i,
                    mu_current,
                    sigma_current,
                    log_posterior_current,
                    acceptances as f64 / (i + 1) as f64
                );
            }
        }

        debug!(
            "Final acceptance rate: {}",
            acceptances as f64 / self.number_of_epochs as f64
        );
        if ret.len() != self.number_of_epochs {
            error!(
                "Error: Returned {} samples instead of {}",
                ret.len(),
                self.number_of_epochs
            );
        }
        ret
    }

    fn log_prior(mu: f64, sigma: f64) -> f64 {
        use std::f64::consts::PI;

        if sigma <= 0.0 {
            return f64::NEG_INFINITY;
        }

        let mu_prior = -0.5 * mu.powi(2) - 0.5 * (2.0 * PI).ln();
        let sigma_prior = -sigma.ln() - 0.5 * sigma.powi(2) - 0.5 * (0.5 * PI).ln(); // Half-normal

        mu_prior + sigma_prior
    }

    pub fn log_normal_distribute(data: &[f64], mu: f64, sigma: f64) -> f64 {
        use std::f64::consts::PI;

        if sigma <= 0.0 || data.iter().any(|&x| x <= 0.0) || data.len() == 0 {
            return f64::NEG_INFINITY;
        }

        let n = data.len() as f64;
        let log_term = n * (0.5 * (2.0 * PI).ln() + sigma.ln());
        let sum_log_x = data.iter().map(|x| x.ln()).sum::<f64>();
        let sum_term = data.iter().map(|x| (x.ln() - mu).powi(2)).sum::<f64>();

        -log_term - sum_log_x - sum_term / (2.0 * sigma * sigma)
    }

    fn log_posterior(&self, data: &[f64], mu: f64, sigma: f64) -> f64 {
        (self.log_prior)(mu, sigma) + (self.log_likelihood)(data, mu, sigma)
    }

    fn calculate_proposal_std_dev(
        &self,
        data: &[f64],
        mu_current: f64,
        sigma_current: f64,
    ) -> (f64, f64) {
        let n = data.len() as f64;
        let log_data: Vec<f64> = data.iter().map(|&x| x.ln()).collect();
        let variance_log = if n > 1.0 {
            log_data
                .iter()
                .map(|&x| (x - mu_current).powi(2)) // Use mu_current instead of sample mean
                .sum::<f64>()
                / (n - 1.0)
        } else {
            1e-6
        };

        let epoch_factor = 1.0 + 10.0 / (1.0 + (self.number_of_epochs as f64).ln());

        let mu_std_dev = (variance_log.sqrt() * epoch_factor * 0.05).clamp(0.02, 2.0); // Reduced from 0.2
        let sigma_std_dev = (sigma_current * epoch_factor * 0.025).clamp(0.02, 2.0); // Reduced from 0.1
        (mu_std_dev, sigma_std_dev)
    }
}

#[cfg(test)]
mod tests {
    use super::*; // Import the MetropolisHasting struct and methods
    use rand::prelude::*;
    use rand_distr::Normal;
    use std::f64::consts::PI;

    // Helper function to generate synthetic log-normal data
    fn generate_log_normal_data(n: usize, mu: f64, sigma: f64, seed: u64) -> Vec<f64> {
        let mut rng = StdRng::seed_from_u64(seed);
        let normal = Normal::new(mu, sigma).unwrap();
        (0..n).map(|_| rng.sample(normal).exp()).collect()
    }

    fn compute_ess(samples: &[f64]) -> f64 {
        let n = samples.len() as f64;
        let mean = samples.iter().sum::<f64>() / n;
        let variance = samples.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let mut acf_sum = 0.0;
        for lag in 1..n as usize {
            let acf = samples
                .iter()
                .skip(lag)
                .zip(samples.iter())
                .map(|(&x, &y)| (x - mean) * (y - mean))
                .sum::<f64>()
                / (n - lag as f64)
                / variance;
            if acf <= 0.0 {
                break;
            }
            acf_sum += acf;
        }
        n / (1.0 + 2.0 * acf_sum)
    }

    // Test log_normal_distribute
    #[test]
    fn test_log_normal_distribute() {
        let data = vec![1.0, 2.71828, 7.38906]; // e^0, e^1, e^2
        let mu = 1.0;
        let sigma = 1.0;

        let result = MetropolisHasting::log_normal_distribute(&data, mu, sigma);
        let n = data.len() as f64;
        let expected_log_term = n * (0.5 * (2.0 * PI).ln() + sigma.ln());
        let expected_sum_log_x = data.iter().map(|x| x.ln()).sum::<f64>();
        let expected_sum_term = data.iter().map(|x| (x.ln() - mu).powi(2)).sum::<f64>();
        let expected =
            -expected_log_term - expected_sum_log_x - expected_sum_term / (2.0 * sigma * sigma);

        assert!(
            (result - expected).abs() < 1e-10,
            "Log-likelihood incorrect: got {}, expected {}",
            result,
            expected
        );

        // Test invalid inputs
        assert_eq!(
            MetropolisHasting::log_normal_distribute(&data, mu, 0.0),
            f64::NEG_INFINITY,
            "Should return NEG_INFINITY for sigma <= 0"
        );
        assert_eq!(
            MetropolisHasting::log_normal_distribute(&[0.0, 1.0], mu, sigma),
            f64::NEG_INFINITY,
            "Should return NEG_INFINITY for non-positive data"
        );
        assert_eq!(
            MetropolisHasting::log_normal_distribute(&[], mu, sigma),
            f64::NEG_INFINITY,
            "Should return NEG_INFINITY for empty data"
        );
    }

    // Test log_prior
    #[test]
    fn test_log_prior() {
        let mu = 0.0;
        let sigma = 1.0;
        let result = MetropolisHasting::log_prior(mu, sigma);
        let expected_mu_prior = -0.5 * mu.powi(2) - 0.5 * (2.0 * PI).ln();
        let expected_sigma_prior = -sigma.ln() - 0.5 * sigma.powi(2) - 0.5 * (0.5 * PI).ln();
        let expected = expected_mu_prior + expected_sigma_prior;

        assert!(
            (result - expected).abs() < 1e-10,
            "Log-prior incorrect: got {}, expected {}",
            result,
            expected
        );

        // Test invalid sigma
        assert_eq!(
            MetropolisHasting::log_prior(mu, 0.0),
            f64::NEG_INFINITY,
            "Should return NEG_INFINITY for sigma <= 0"
        );
        assert_eq!(
            MetropolisHasting::log_prior(mu, -1.0),
            f64::NEG_INFINITY,
            "Should return NEG_INFINITY for negative sigma"
        );
    }

    #[test]
    fn test_metropolis_hastings() {
        let mu_true = 0.5;
        let sigma_true = 0.8;
        let data = generate_log_normal_data(100000, mu_true, sigma_true, 42);
        let mh = MetropolisHasting::new(
            10000,
            MetropolisHasting::log_prior,
            MetropolisHasting::log_normal_distribute,
        );

        let samples = mh.metropolis_hastings(&data, 0.4, 0.9, 42);
        assert_eq!(samples.len(), 10000, "Should return 200000 samples");

        let burn_in = 4000;
        let mu_samples: Vec<f64> = samples.iter().skip(burn_in).map(|&(mu, _)| mu).collect();
        let sigma_samples: Vec<f64> = samples
            .iter()
            .skip(burn_in)
            .map(|&(_, sigma)| sigma)
            .collect();

        assert!(!mu_samples.is_empty(), "Mu samples should not be empty");
        assert!(
            !sigma_samples.is_empty(),
            "Sigma samples should not be empty"
        );

        let mu_mean = mu_samples.iter().sum::<f64>() / mu_samples.len() as f64;
        let sigma_mean = sigma_samples.iter().sum::<f64>() / sigma_samples.len() as f64;

        let mu_var = if mu_samples.len() > 1 {
            mu_samples
                .iter()
                .map(|&x| (x - mu_mean).powi(2))
                .sum::<f64>()
                / (mu_samples.len() - 1) as f64
        } else {
            0.0
        };
        let sigma_var = if sigma_samples.len() > 1 {
            sigma_samples
                .iter()
                .map(|&x| (x - sigma_mean).powi(2))
                .sum::<f64>()
                / (sigma_samples.len() - 1) as f64
        } else {
            0.0
        };
        let mu_std_err = mu_var.sqrt() / (mu_samples.len() as f64).sqrt();
        let sigma_std_err = sigma_var.sqrt() / (sigma_samples.len() as f64).sqrt();

        let mut mu_acf_sum = 0.0;
        for lag in 1..100 {
            let acf = mu_samples
                .iter()
                .skip(lag)
                .zip(mu_samples.iter())
                .map(|(&x, &y)| (x - mu_mean) * (y - mu_mean))
                .sum::<f64>()
                / (mu_samples.len() - lag) as f64
                / mu_var;
            if acf <= 0.0 || !acf.is_finite() {
                break;
            }
            mu_acf_sum += acf;
        }
        let mu_ess = if mu_acf_sum.is_finite() && mu_acf_sum > 0.0 {
            mu_samples.len() as f64 / (1.0 + 2.0 * mu_acf_sum)
        } else {
            mu_samples.len() as f64
        };
        let mu_std_err_adjusted = if mu_ess > 0.0 {
            mu_var.sqrt() / mu_ess.sqrt()
        } else {
            mu_std_err
        };

        assert!(
            (mu_mean - mu_true).abs() < 5.0 * mu_std_err_adjusted,
            "Mu mean {} too far from true value {}, std_err_adjusted={} while diff is {}",
            mu_mean,
            mu_true,
            mu_std_err_adjusted,
            (mu_mean - mu_true).abs(),
        );
        assert!(
            (sigma_mean - sigma_true).abs() < 5.0 * sigma_std_err,
            "Sigma mean {} too far from true value {}, std_err={} while diff is {}",
            sigma_mean,
            sigma_true,
            sigma_std_err,
            (sigma_mean - sigma_true).abs(),
        );

        eprintln!(
            "Mu mean: {}, Sigma mean: {}, Mu ESS: {}",
            mu_mean, sigma_mean, mu_ess
        );
    }

    // Test numerical stability with extreme data
    #[test]
    fn test_metropolis_hastings_extreme_data() {
        let data = vec![1e-10, 1e10]; // Extreme values
        let mh = MetropolisHasting::new(
            1000,
            MetropolisHasting::log_prior,
            MetropolisHasting::log_normal_distribute,
        );

        let samples = mh.metropolis_hastings(&data, 0.0, 1.0, 42);
        assert_eq!(samples.len(), 1000, "Should return 1000 samples");
        assert!(
            samples
                .iter()
                .all(|&(mu, sigma)| mu.is_finite() && sigma.is_finite() && sigma > 0.0),
            "Samples should be finite and sigma positive"
        );
    }
}
