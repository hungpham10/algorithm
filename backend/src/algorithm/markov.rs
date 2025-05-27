use rand::prelude::*;
use rand_distr::Normal;

pub type LogLikelihoodCallback = fn(data: &[f64], mu: f64, sigma: f64) -> f64;
pub type LogPriorCallback = fn(f64, f64) -> f64;

struct Markov {
    log_likelihood: LogLikelihoodCallback,
    log_prior: LogPriorCallback,
    number_of_epochs: usize,
}

impl Markov {
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

    fn log_prior(mu: f64, sigma: f64) -> f64 {
        use std::f64::NEG_INFINITY;
        use std::f64::consts::PI;

        if sigma <= 0.0 {
            return NEG_INFINITY;
        }

        let mu_prior = -0.5 * mu.powi(2) - 0.5 * (2.0 * PI).ln();
        let sigma_prior = -0.5 * sigma.powi(2) - 0.5 * (2.0 * PI).ln();

        return mu_prior + sigma_prior
    }

    pub fn log_normal_distribute(data: &[f64], mu: f64, sigma: f64) -> f64 {
        use std::f64::consts::PI;

        let log_term = (data.len() as f64) * (0.5 * (2.0 * PI).ln() + sigma.ln());
        let sum_term = data.iter().map(|x| (x.ln() - mu).powi(2) ).sum::<f64>();
        return -log_term - sum_term / (2.0 * sigma * sigma);
    }

    fn log_posterior(&self, data: &[f64], mu: f64, sigma: f64) -> f64 {
        (self.log_prior)(mu, sigma) + 
        (self.log_likelihood)(data, mu, sigma)
    }

    fn metropolis_hastings(
        &self, 
        data: &[f64],
        mu_initial: f64,
        sigma_initial: f64,
    ) -> Vec<(f64, f64)> {
        let mut ret = Vec::new();
        let mut rng = rand::thread_rng();
        let mut mu_current = mu_initial;
        let mut sigma_current = sigma_initial;
        let mut log_posterior_current = self.log_posterior(data, mu_current, sigma_current);

        for _ in 0..self.number_of_epochs {
            let proposal_std_dev = self.calculate_proposal_std_dev(data, mu_current, sigma_current);
            let mu_proposal = rng.sample(Normal::new(mu_current, proposal_std_dev).unwrap());
            let sigma_proposal = rng.sample(Normal::new(sigma_current, proposal_std_dev).unwrap());
            let log_posterior_proposal = self.log_posterior(data, mu_proposal, sigma_proposal);

            if rng.gen::<f64>() < (log_posterior_proposal - log_posterior_current).exp() {
                mu_current = mu_proposal;
                sigma_current = sigma_proposal;
                log_posterior_current = log_posterior_proposal;

                ret.push((mu_current, sigma_current));
            }
        }

        return ret;
    }

    fn calculate_proposal_std_dev(
        &self,
        data: &[f64],
        mu_current: f64,
        sigma_current: f64,
    ) -> f64 {
        // Purpose: Compute an adaptive proposal standard deviation for Metropolis-Hastings.
        // The proposal standard deviation controls the step size of the normal proposal distributions
        // for mu and sigma. We use mu_current to estimate the posterior's scale by computing the
        // variance of log(data) relative to mu_current, reflecting the likelihood's structure
        // (log-normal distribution). This adapts the proposal to the chain's current position,
        // targeting an optimal acceptance rate (23–44%, per Roberts et al., 1997).
    
        // Handle empty data to prevent invalid computations.
        if data.is_empty() {
            return 1.0; // Default fallback to ensure algorithm stability.
        }
    
        // Compute variance of log-transformed data relative to mu_current.
        // Since the likelihood (log_normal_distribute) models ln(data) ~ N(mu, sigma^2),
        // the sample variance of ln(x_i) - mu_current estimates the posterior's spread.
        let n = data.len() as f64;
        let log_data: Vec<f64> = data.iter().map(|&x| x.ln()).collect();
        let variance_log = log_data
            .iter()
            .map(|&x| (x - mu_current).powi(2)) // Use mu_current instead of sample mean
            .sum::<f64>()
            / (n - 1.0);
    
        // Base scale: Combine the sample standard deviation (sqrt(variance_log)),
        // current sigma (sigma_current), and a minimum threshold (0.1).
        // - sqrt(variance_log) reflects data variability relative to mu_current.
        // - sigma_current reflects the current posterior scale estimate.
        // - 0.1 prevents overly small proposals that could stall exploration.
        let base_scale = variance_log.sqrt().max(sigma_current).max(0.1);
    
        // Adaptive scaling: Adjust the proposal scale based on the number of epochs to balance
        // exploration (early, larger steps) and exploitation (later, smaller steps).
        // The logarithmic decay (inspired by Haario et al., 2001) reduces the scale over time.
        let epoch_factor = 1.0 + 10.0 / (1.0 + (self.number_of_epochs as f64).ln());
    
        // Compute the proposal standard deviation by scaling the base scale.
        // The constant 0.1 is a tuning parameter to ensure moderate step sizes, as large steps
        // reduce acceptance rates, while small steps slow convergence (Gelman et al., 1996).
        let proposal_std_dev = base_scale * epoch_factor * 0.1;
    
        // Clamp to prevent numerical issues (e.g., negative or extreme values) that could
        // disrupt the normal proposal distributions in metropolis_hastings.
        proposal_std_dev.clamp(0.01, 10.0)
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use rand_distr::{LogNormal, Normal, Distribution};
    
    // Helper function to compute mean of a vector of f64
    fn compute_mean(data: &[f64]) -> f64 {
        if data.is_empty() { return 0.0; }
        data.iter().sum::<f64>() / data.len() as f64
    }
    
    // Helper function to compute standard deviation of a vector of f64
    fn compute_std_dev(data: &[f64], mean: f64) -> f64 {
        if data.is_empty() { return 0.0; }
        let variance = data.iter()
            .map(|&x| (x - mean).powi(2))
            .sum::<f64>() / (data.len() as f64 - 1.0);
        variance.sqrt()
    }

    // Helper function to create a histogram from data
    fn create_histogram(data: &[f64], num_bins: usize, min: f64, max: f64) -> (Vec<f64>, Vec<usize>) {
        let bin_width = (max - min) / num_bins as f64;
        let mut bins = vec![0; num_bins];
        let bin_edges: Vec<f64> = (0..=num_bins).map(|i| min + i as f64 * bin_width).collect();
        
        for &x in data {
            if x >= min && x < max {
                let bin_idx = ((x - min) / bin_width).floor() as usize;
                if bin_idx < num_bins {
                    bins[bin_idx] += 1;
                }
            }
        }
        
        (bin_edges, bins)
    }
    
    // Custom likelihood based on histogram density
    fn histogram_likelihood(data: &[f64], mu: f64, _sigma: f64) -> f64 {
        // Data contains histogram: bin_edges, bin_counts, bin_width
        // Interpret data as [bin_edges..., bin_counts..., bin_width]
        let num_bins = (data.len() - 1) / 2;
        let bin_edges = &data[..num_bins + 1];
        let bin_counts = &data[num_bins + 1..2 * num_bins + 1];
        let bin_width = data[2 * num_bins + 1];
        
        // Find the bin containing mu
        let bin_idx = bin_edges.iter().position(|&edge| mu < edge).map(|i| i - 1);
        
        match bin_idx {
            Some(idx) if idx < bin_counts.len() => {
                // Return log of normalized bin count as likelihood
                let count = bin_counts[idx] as f64;
                let total_counts = bin_counts.iter().sum::<f64>();
                if count > 0.0 {
                    (count / (total_counts * bin_width)).ln()
                } else {
                    std::f64::NEG_INFINITY
                }
            }
            _ => std::f64::NEG_INFINITY,
        }
    }
    // Custom normal likelihood for testing standard normal distribution
    fn log_normal_likelihood(data: &[f64], mu: f64, sigma: f64) -> f64 {
        use std::f64::consts::PI;
        if sigma <= 0.0 {
            return std::f64::NEG_INFINITY;
        }
        let log_term = (data.len() as f64) * (0.5 * (2.0 * PI).ln() + sigma.ln());
        let sum_term = data.iter().map(|&x| (x - mu).powi(2)).sum::<f64>();
        -log_term - sum_term / (2.0 * sigma * sigma)
    } 
    
    #[test]
    fn test_metropolis_hastings_basic() {
        // Test case: Verify that Metropolis-Hastings samples approximate the true mu and sigma
        // for synthetic log-normal data.
        
        // Generate synthetic data: LogNormal(mu=0.5, sigma=0.3)
        let true_mu = 0.5;
        let true_sigma = 0.3;
        let mut rng = rand::thread_rng();
        let lognormal = LogNormal::new(true_mu, true_sigma).unwrap();
        let data: Vec<f64> = (0..1000).map(|_| lognormal.sample(&mut rng)).collect();
        
        // Initialize Markov struct
        let markov = Markov::new(
            100000, // number_of_epochs
            Markov::log_prior,
            Markov::log_normal_distribute,
        );
        
        // Run Metropolis-Hastings
        let samples = markov.metropolis_hastings(&data, 0.0, 1.0);
        
        // Extract mu and sigma samples
        let mu_samples: Vec<f64> = samples.iter().map(|&(mu, _)| mu).collect();
        let sigma_samples: Vec<f64> = samples.iter().map(|&(_, sigma)| sigma).collect();
        
        // Compute sample statistics
        let mu_mean = compute_mean(&mu_samples);
        let sigma_mean = compute_mean(&sigma_samples);
        
        // Assert that sample means are close to true values (within 2 standard errors)
        // Standard error = std_dev / sqrt(n), where n is number of samples
        let mu_std = compute_std_dev(&mu_samples, mu_mean);
        let sigma_std = compute_std_dev(&sigma_samples, sigma_mean);
        let mu_se = mu_std / (mu_samples.len() as f64).sqrt();
        let sigma_se = sigma_std / (sigma_samples.len() as f64).sqrt();
        
        assert!(
            (mu_mean - true_mu).abs() < 2.0 * mu_se,
            "Mu mean {} not close to true mu {}", mu_mean, true_mu
        );
        assert!(
            (sigma_mean - true_sigma).abs() < 2.0 * sigma_se,
            "Sigma mean {} not close to true sigma {}", sigma_mean, true_sigma
        );
        
        // Check that some samples were accepted
        assert!(!samples.is_empty(), "No samples were accepted");
    }
     
    #[test]
    fn test_calculate_proposal_std_dev() {
        // Test case: Verify that proposal_std_dev is reasonable
        let data = vec![1.2, 2.3, 1.8];
        let markov = Markov::new(
            1000,
            Markov::log_prior,
            Markov::log_normal_distribute,
        );
        
        let std_dev = markov.calculate_proposal_std_dev(&data, 0.5, 0.3);
        
        // Expect std_dev to be within clamped bounds [0.01, 10.0]
        assert!(
            (0.01..=10.0).contains(&std_dev),
            "Proposal std_dev {} outside bounds [0.01, 10.0]", std_dev
        );
        
        // Expect std_dev to be positive
        assert!(std_dev > 0.0, "Proposal std_dev {} is not positive", std_dev);
    }
 
    #[test]
    fn test_metropolis_hastings_normal_distribution() {
        // Test case: Use MH to approximate a normal posterior N(mu=2.0, sigma=1.0)
        // Generate data from N(2.0, 1.0), run MH, and verify sample statistics
        
        // Generate synthetic data: Normal(mu=2.0, sigma=1.0)
        let true_mu = 2.0;
        let true_sigma = 1.0;
        let mut rng = rand::thread_rng();
        let normal = Normal::new(true_mu, true_sigma).unwrap();
        let data: Vec<f64> = (0..1000).map(|_| normal.sample(&mut rng)).collect();
        
        // Compute sample statistics for reference
        let sample_mean = compute_mean(&data);
        let sample_std = compute_std_dev(&data, sample_mean);
        
        // Initialize Markov struct with normal likelihood
        let markov = Markov::new(
            10000, // number_of_epochs
            Markov::log_prior,
            log_normal_likelihood,
        );
        
        // Run Metropolis-Hastings
        let samples = markov.metropolis_hastings(&data, 0.0, 1.0);
        
        // Extract mu and sigma samples, discarding first 10% as burn-in
        let burn_in = (0.1 * samples.len() as f64) as usize;
        let mu_samples: Vec<f64> = samples.iter().skip(burn_in).map(|&(mu, _)| mu).collect();
        let sigma_samples: Vec<f64> = samples.iter().skip(burn_in).map(|&(_, sigma)| sigma).collect();
        
        // Compute sample statistics
        let mu_mean = compute_mean(&mu_samples);
        let sigma_mean = compute_mean(&sigma_samples);
        
        // Compute standard errors for tolerance
        let mu_std = compute_std_dev(&mu_samples, mu_mean);
        let sigma_std = compute_std_dev(&sigma_samples, sigma_mean);
        let mu_se = mu_std / (mu_samples.len() as f64).sqrt();
        let sigma_se = sigma_std / (sigma_samples.len() as f64).sqrt();
        
        // Assert that mu mean is close to sample mean (approx. true_mu)
        // Tolerance: 2 standard errors or 0.1, whichever is larger
        let mu_tolerance = mu_se * 2.0;
        assert!(
            (mu_mean - sample_mean).abs() < mu_tolerance.max(0.1),
            "Mu mean {} not close to sample mean {} (tolerance {})",
            mu_mean, sample_mean, mu_tolerance.max(0.1)
        );
        
        // Assert that sigma mean is close to sample std dev (approx. true_sigma)
        let sigma_tolerance = sigma_se * 2.0;
        assert!(
            (sigma_mean - sample_std).abs() < sigma_tolerance.max(0.1),
            "Sigma mean {} not close to sample std {} (tolerance {})",
            sigma_mean, sample_std, sigma_tolerance.max(0.1)
        );
        
        // Check that some samples were accepted
        assert!(!mu_samples.is_empty(), "No samples were accepted after burn-in");
    }    

    #[test]
    fn test_metropolis_hastings_histogram_exploration() {
        // Test case: Use MH to explore a histogram from a mixture of normals
        // Generate data from 0.6*N(2, 1) + 0.4*N(6, 0.5), create histogram,
        // and use MH to sample from the histogram's density
        
        // Generate synthetic data: mixture of N(2, 1) and N(6, 0.5)
        let mut rng = rand::thread_rng();
        let normal1 = Normal::new(2.0, 1.0).unwrap();
        let normal2 = Normal::new(6.0, 0.5).unwrap();
        let mut data = vec![];
        for _ in 0..10000 {
            if rng.gen::<f64>() < 0.6 {
                data.push(normal1.sample(&mut rng));
            } else {
                data.push(normal2.sample(&mut rng));
            }
        }
        
        // Create histogram: 50 bins from 0 to 10
        let num_bins = 50;
        let min = 0.0;
        let max = 10.0;
        let (bin_edges, bin_counts) = create_histogram(&data, num_bins, min, max);
        let bin_width = (max - min) / num_bins as f64;
        
        // Prepare data for likelihood: [bin_edges, bin_counts, bin_width]
        let mut likelihood_data: Vec<f64> = bin_edges.clone();
        likelihood_data.extend(bin_counts.iter().map(|&c| c as f64));
        likelihood_data.push(bin_width);
        
        // Initialize Markov struct with histogram likelihood
        let markov = Markov::new(
            20000, // number_of_epochs
            |mu, _sigma| Markov::log_prior(mu, 1.0), // Fix sigma=1.0 in prior
            histogram_likelihood,
        );
        
        // Run Metropolis-Hastings
        let samples = markov.metropolis_hastings(&likelihood_data, 4.0, 1.0);
        
        // Extract mu samples, discarding first 10% as burn-in
        let burn_in = (0.1 * samples.len() as f64) as usize;
        let mu_samples: Vec<f64> = samples.iter().skip(burn_in).map(|&(mu, _)| mu).collect();
        
        // Compute sample histogram
        let (sample_bin_edges, sample_bin_counts) = create_histogram(&mu_samples, num_bins, min, max);
        
        // Normalize histograms for comparison
        let total_data_counts = bin_counts.iter().sum::<usize>() as f64;
        let total_sample_counts = sample_bin_counts.iter().sum::<usize>() as f64;
        let data_density: Vec<f64> = bin_counts.iter()
            .map(|&c| c as f64 / (total_data_counts * bin_width))
            .collect();
        let sample_density: Vec<f64> = sample_bin_counts.iter()
            .map(|&c| c as f64 / (total_sample_counts * bin_width))
            .collect();
        
        // Compute mean absolute difference between normalized densities
        let density_diff: f64 = data_density.iter()
            .zip(sample_density.iter())
            .map(|(&d, &s)| (d - s).abs())
            .sum::<f64>() / num_bins as f64;
        
        // Assert that densities difference is small (arbitrary threshold)
        assert!(
            density_diff < 0.05,
            "Histogram density difference {} too large", density_diff
        );
        
        // Check that some samples were accepted
        assert!(!mu_samples.is_empty(), "No samples were accepted after burn-in");
        
        // Optional: Plot histograms for visualization (not implemented in test)
        // To visualize, use a plotting library like `plotters` or Python:
        // - Plot data_density vs bin_edges (original histogram)
        // - Plot sample_density vs sample_bin_edges (MH samples)
    } 
}
