use rand::prelude::*;
use rand_distr::{Normal, Distribution};


pub type LogLikelihoodCallback = fn(data: &[f64], mu: f64, sigma: f64) -> f64;
pub type LogPriorCallback = fn(f64, f64) -> f64;

struct Bayesian {
    log_likelihood: LogLikelihoodCallback,
    log_prior: LogPriorCallback,
    number_of_epochs: usize,
}

impl Bayesian {
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

    pub fn log_prior(mu: f64, sigma: f64) -> f64 {
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
        (self.log_prior)(mu, sigma) + (self.log_likelihood)(data, mu, sigma)
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
        // @TODO: calculate by using 
        return 10.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    
}
