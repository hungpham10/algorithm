use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex, RwLock};

use nalgebra::{DMatrix, DVector};
use rand::Rng;
use rand_distr::{Distribution, StandardNormal};

use anyhow::{anyhow, Result};
use reqwest;

use vnscope::algorithm::genetic::{Genetic, Individual, InfluxDb, Model, Player};
use vnscope::schemas::CandleStick;

use crate::api::ohcl::v1::OhclResponse;

#[derive(Clone, Copy)]
enum Phase {
    Test,
    Train,
}

#[derive(Clone)]
struct Data {
    candles: Arc<Vec<CandleStick>>,
    range: usize,
    begin: usize,
    split: usize,
    end: usize,
}

impl Data {
    fn new(candles: Arc<Vec<CandleStick>>, range: usize) -> Self {
        Self {
            candles,
            range,
            begin: 0,
            split: 0,
            end: 0,
        }
    }

    fn shuttle(&mut self) {
        let len = self.candles.len();
        let min_required = self.range * 2;
        if len < min_required {
            self.begin = 0;
            self.end = len;
            self.split = len / 2;
            return;
        }
        let mut rng = rand::thread_rng();
        let end = rng.gen_range(min_required..=len);
        let max_begin = end.saturating_sub(min_required);
        let begin = rng.gen_range(0..=max_begin);
        let min_split = begin.saturating_add(self.range);
        let max_split = end.saturating_sub(self.range);
        let split = if min_split <= max_split {
            rng.gen_range(min_split..=max_split)
        } else {
            begin
        };
        self.begin = begin;
        self.split = split;
        self.end = end;
    }

    fn window(&self) -> usize {
        self.range
    }

    fn size(&self, phase: &Phase) -> usize {
        match phase {
            Phase::Train => self
                .split
                .saturating_sub(self.begin)
                .saturating_sub(self.range),
            Phase::Test => self
                .end
                .saturating_sub(self.split)
                .saturating_sub(self.range),
        }
    }

    fn sample(&self, i: usize, phase: &Phase) -> Result<&[CandleStick]> {
        let start = match phase {
            Phase::Train => self.begin.saturating_add(i),
            Phase::Test => self.split.saturating_add(i),
        };
        let end_slice = start.saturating_add(self.range);

        if self.size(phase) > i {
            match phase {
                Phase::Train => {
                    if end_slice <= self.candles.len() {
                        Ok(&self.candles[start..end_slice])
                    } else {
                        Err(anyhow!("out of range"))
                    }
                }
                Phase::Test => {
                    if end_slice <= self.candles.len() {
                        Ok(&self.candles[start..end_slice])
                    } else {
                        Err(anyhow!("out of range"))
                    }
                }
            }
        } else {
            Err(anyhow!("out of range"))
        }
    }

    fn last_candle(&self, i: usize, phase: &Phase) -> Result<CandleStick> {
        let idx = match phase {
            Phase::Train => self.begin,
            Phase::Test => self.split,
        }
        .saturating_add(i)
        .saturating_add(self.range);

        if self.size(phase) > i {
            match phase {
                Phase::Train => {
                    if idx < self.candles.len() {
                        Ok(self.candles[idx].clone())
                    } else {
                        Err(anyhow!("out of range"))
                    }
                }
                Phase::Test => {
                    if idx < self.candles.len() {
                        Ok(self.candles[idx].clone())
                    } else {
                        Err(anyhow!("out of range"))
                    }
                }
            }
        } else {
            Err(anyhow!("out of range"))
        }
    }
}

#[derive(Clone)]
struct Investor {
    // @NOTE: shared arguments
    data: Arc<RwLock<Data>>,
    phase: Arc<RwLock<Phase>>,

    // @NOTE: factors
    factors: DVector<f64>,
    // @NOTE: configuration
    fee: f64,
    initialize_money: f64,
    initialize_stock: f64,
    holding_period: usize,
    minimum_buy: usize,
}

impl Investor {
    pub fn new(
        data: Arc<RwLock<Data>>,
        phase: Arc<RwLock<Phase>>,
        money: f64,
        stock: f64,
        stock_holding_period: usize,
        minimum_stock_buy: usize,
        factors: DVector<f64>,
    ) -> Self {
        Self {
            data,
            factors,
            phase,
            fee: 0.001,
            initialize_money: money,
            initialize_stock: stock,
            holding_period: stock_holding_period,
            minimum_buy: minimum_stock_buy,
        }
    }

    fn logit(&self, candles: &[CandleStick]) -> f64 {
        let window = candles.len();
        if window == 0 {
            return 0.0;
        }

        let mut sum = 0.0;
        for i in 0..window {
            let candle = &candles[i];
            let prev_candle = if i == 0 { candle } else { &candles[i - 1] };

            let delta_o = (candle.o - prev_candle.c) / prev_candle.c;
            let delta_h = (candle.h - prev_candle.c) / prev_candle.c;
            let delta_c = (candle.c - prev_candle.c) / prev_candle.c;
            let delta_l = (candle.l - prev_candle.c) / prev_candle.c;
            let v_scaled = candle.v / 10_000_000.0;

            sum += self.factors[5 * i] * delta_o
                + self.factors[5 * i + 1] * delta_h
                + self.factors[5 * i + 2] * delta_c
                + self.factors[5 * i + 3] * delta_l
                + self.factors[5 * i + 4] * v_scaled;
        }

        let normalized = sum / (window as f64).sqrt();
        let bias = self.factors.iter().last().copied().unwrap_or(0.0);

        (normalized + bias).clamp(-3.0, 3.0)
    }

    fn sigmoid(x: f64) -> f64 {
        1.0 / (1.0 + (-x).exp())
    }
}

impl Player for Investor {
    fn initialize(&mut self) -> Result<()> {
        Ok(())
    }

    fn estimate(&self) -> Result<f64> {
        let mut money = self.initialize_money;
        let mut stock = self.initialize_stock;

        let phase = self
            .phase
            .read()
            .map_err(|error| anyhow!("Failed read phase: {}", error))?;
        let data = self
            .data
            .read()
            .map_err(|error| anyhow!("Failed read data: {}", error))?;

        for i in 0..data.size(&phase) {
            let candles_window = data.sample(i, &phase)?;
            let p_buy = Self::sigmoid(self.logit(candles_window));

            if p_buy > 0.75 {
                let buy_candle = data.last_candle(i, &phase)?;
                if money > (self.minimum_buy as f64) * buy_candle.c {
                    money -= (1.0 + self.fee) * self.minimum_buy as f64 * buy_candle.c;
                    stock += self.minimum_buy as f64;

                    if i + self.holding_period < data.size(&phase) {
                        let sell_candle = data.last_candle(i + self.holding_period, &phase)?;
                        money += (1.0 - self.fee) * self.minimum_buy as f64 * sell_candle.o;
                        stock -= self.minimum_buy as f64;
                    }
                }
            }
        }

        let last_price = data.last_candle(data.size(&phase) - 1, &phase)?.c;
        Ok((money + stock * last_price - self.initialize_money) / self.initialize_money)
    }

    fn gene(&self) -> DVector<f64> {
        self.factors.clone()
    }
}

struct Spot {
    // @NOTE: For stock modeling
    data: Arc<RwLock<Data>>,
    phase: Arc<RwLock<Phase>>,
    money: f64,
    stock: f64,
    lifespan: i64,
    stock_holding_period: usize,
    minimum_stock_buy: usize,

    // @NOTE: CMA-ES
    mean: DVector<f64>,
    sigma: f64,
    cov_matrix: DMatrix<f64>,
    p_sigma: DVector<f64>,
    p_c: DVector<f64>,
    chi_n: f64,
    mu: usize,
    weights: DVector<f64>,
}

impl Spot {
    fn new(
        data: Arc<RwLock<Data>>,
        money: f64,
        stock: f64,
        lifespan: i64,
        stock_holding_period: usize,
        minimum_stock_buy: usize,
    ) -> Result<Self> {
        let mut rng = rand::thread_rng();

        data.write()
            .map_err(|error| anyhow!("Failed to read data: {}", error))?
            .shuttle();

        // @TODO: cần điều chỉnh lại hằng số này để khống chế số lượng
        //        factors mỗi candles
        let n = 5 * data
            .read()
            .map_err(|error| anyhow!("Failed to read data: {}", error))?
            .window();

        // CÁC THAM SỐ CMA-ES TIÊU CHUẨN DỰA TRÊN N
        let lambda = 4 + (3.0 * (n as f64).ln()).floor() as usize; // Population size
        let mu = lambda / 2;

        // Tính toán Trọng số w_i (logarithmic weights)
        let weights_vec = (0..mu)
            .map(|i| (mu as f64).ln() - ((i as f64) + 0.5).ln())
            .collect::<Vec<_>>();
        let sum_w: f64 = weights_vec.iter().sum();
        let weights = DVector::from_vec(weights_vec.iter().map(|w| w / sum_w).collect());

        // Expected value of ||N(0, I)|| (Hệ số giảm chấn)
        let chi_n =
            (n as f64).sqrt() * (1.0 - 1.0 / (4.0 * n as f64) + 1.0 / (21.0 * (n as f64).powi(2)));

        Ok(Self {
            data,
            money,
            stock,
            lifespan,
            stock_holding_period,
            minimum_stock_buy,

            //
            phase: Arc::new(RwLock::new(Phase::Train)),

            // Khởi tạo trạng thái
            mean: DVector::from_iterator(n, (0..n).map(|_| rng.gen_range(-1.0..1.0))),
            sigma: 0.5,
            cov_matrix: DMatrix::identity(n, n),

            p_sigma: DVector::zeros(n),
            p_c: DVector::zeros(n),

            chi_n,
            mu,
            weights,
        })
    }
}

impl Model<Investor> for Spot {
    fn random(&self) -> Result<Investor> {
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

        Ok(Investor::new(
            self.data.clone(),
            self.phase.clone(),
            self.money,
            self.stock,
            self.stock_holding_period,
            self.minimum_stock_buy,
            DVector::from_iterator(num_factors, new_factors.iter().map(|&x| x.clamp(-1.0, 1.0))),
        ))
    }

    fn mutate(&self, item: &mut Investor, _arguments: &Vec<f64>, index: usize) -> Result<()> {
        let mut rng = rand::thread_rng();
        let noise = rng.gen_range(-0.1..0.1);
        item.factors[index] += noise;
        item.factors[index] = item.factors[index].clamp(-1.0, 1.0);
        Ok(())
    }

    fn crossover(&self, father: &Investor, mother: &Investor) -> Result<Investor> {
        let mut rng = rand::thread_rng();
        let factors_vec: Vec<f64> = father
            .factors
            .iter()
            .zip(mother.factors.iter())
            .map(|(f, m)| {
                let base = if rng.gen_bool(0.5) { *f } else { *m };
                (base + rng.gen_range(-0.05..0.05)).clamp(-1.0, 1.0)
            })
            .collect();

        Ok(Investor::new(
            self.data.clone(),
            self.phase.clone(),
            self.money,
            self.stock,
            self.stock_holding_period,
            self.minimum_stock_buy,
            DVector::from(factors_vec),
        ))
    }

    fn extinguish(&self, item: &Individual<Investor>) -> Result<bool> {
        Ok(item.lifetime() > self.lifespan)
    }

    fn optimize(&mut self, population: &Vec<Individual<Investor>>) -> Result<()> {
        let n = self.mean.len();
        if n == 0 || population.is_empty() {
            return Err(anyhow!("Failed duo to population is emptied"));
        }

        // @NOTE: lock old phase
        let old_phase = *self
            .phase
            .read()
            .map_err(|error| anyhow!("Failed read phase: {}", error))?;

        // @NOTE: move to test phase
        *self
            .phase
            .write()
            .map_err(|error| anyhow!("Failed write phase: {}", error))? = Phase::Test;

        // @TODO: Review performance using test data

        let result = (|| -> Result<()> {
            // @NOTE: step 1 select best individuo
            let mut sorted_population = population.clone();
            sorted_population.sort_by(|a, b| {
                b.estimate()
                    .partial_cmp(&a.estimate())
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let mu = self.mu.min(sorted_population.len());
            let best_individuals = &sorted_population[0..mu];

            let mut y_k: Vec<DVector<f64>> = Vec::with_capacity(mu);

            for individual in best_individuals.iter() {
                let x = individual.player().gene();
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
        })();

        *self
            .phase
            .write()
            .map_err(|error| anyhow!("Failed write phase: {}", error))? = old_phase;
        result
    }
}

struct Simulator {
    genetic: Option<Arc<Mutex<Genetic<Investor, Spot>>>>,
    candles: Option<Vec<CandleStick>>,
    money: Option<f64>,
    stock: Option<f64>,
    arguments: Vec<Vec<f64>>,
    pmutation: f64,
    session: i64,
    stock_holding_period: usize,
    minimum_stock_buy: usize,
}

impl Simulator {
    pub fn new() -> Self {
        Self {
            candles: None,
            genetic: None,
            money: None,
            stock: None,
            session: 0,
            arguments: Vec::new(),
            pmutation: 0.1,
            minimum_stock_buy: 100,
            stock_holding_period: 10,
        }
    }

    pub fn with_minimum_stock_buy(&mut self, value: usize) {
        self.minimum_stock_buy = value
    }

    pub fn with_stock_holding_period(&mut self, value: usize) {
        self.stock_holding_period = value
    }

    pub fn with_arguments(&mut self, arguments: Vec<Vec<f64>>) {
        self.arguments = arguments;
    }

    pub fn with_money(&mut self, money: f64) {
        self.money = Some(money);
    }

    pub fn with_stock(&mut self, stock: f64) {
        self.stock = Some(stock);
    }

    pub async fn with_sampling(
        &mut self,
        provider: &str,
        market: &str,
        symbol: &str,
        resolution: &str,
        from: i64,
        to: i64,
    ) -> Result<()> {
        let resp = reqwest::get(format!(
            "https://{}/api/investing/v1/ohcl/{}/{}?resolution={}&from={}&to={}&limit=0",
            provider, market, symbol, resolution, from, to,
        ))
        .await?;
        self.candles = Some(
            resp.json::<OhclResponse>()
                .await
                .map_err(|error| anyhow!("Failed parsing candlesticks: {:?}", error))?
                .ohcl
                .unwrap_or(Vec::new()),
        );
        Ok(())
    }

    pub async fn with_genetic(
        &mut self,
        capacity: usize,
        n_loop: usize,
        n_train: usize,
        n_break: usize,
        d_range: usize,
        shuttle_rate: f64,
        influxdb: Option<InfluxDb>,
    ) -> Result<()> {
        if self.genetic.is_none() {
            let candles = self.candles.clone().ok_or(anyhow!(
                "Not found candles, please call with_sampling first"
            ))?;

            self.genetic = Some(Arc::new(Mutex::new(Genetic::new(
                capacity,
                Arc::new(RwLock::new(Spot::new(
                    Arc::new(RwLock::new(Data::new(Arc::new(candles), d_range))),
                    self.money.ok_or(anyhow!("Not found money"))?,
                    self.stock.unwrap_or(0.0),
                    30,
                    self.stock_holding_period,
                    self.minimum_stock_buy,
                )?)),
                influxdb,
            ))));
        }

        let mut genetic = self
            .genetic
            .as_ref()
            .unwrap()
            .lock()
            .map_err(|error| anyhow!("Failed to lock genetic: {}", error))?;
        let mut step_cnt = 0;
        let mut breaking_cnt = 0;
        let mut previous_p55 = 0.0;
        let mut previous_diff_p55 = 0.0;

        if self.session == 0 {
            genetic.initialize(capacity, self.session, Some(shuttle_rate))?;
        }

        for _ in 0..n_loop {
            for i in 0..n_train {
                genetic.evolute(capacity / 5, self.session + (i + 1) as i64, self.pmutation)?;

                let stats = genetic.statistic(self.session + (i + 1) as i64).await?;
                let current_p55 = stats.p55;
                let current_diff_p55 = current_p55 - previous_p55;

                if current_p55 <= previous_p55 {
                    breaking_cnt += 1;
                } else if current_diff_p55 <= previous_diff_p55 {
                    breaking_cnt += 1;
                } else {
                    breaking_cnt = 0;
                }

                println!(
                    "session={}, p99={}, p95={}, p75={}, p55={}",
                    self.session + (i + 1) as i64,
                    stats.p99,
                    stats.p95,
                    stats.p75,
                    stats.p55,
                );
                if breaking_cnt > n_break {
                    break;
                }

                step_cnt += 1;
                previous_p55 = current_p55;
                previous_diff_p55 = current_diff_p55;

                if i + 1 < n_loop {
                    genetic.fluctuate(
                        self.session + (i + 1) as i64,
                        &self.arguments,
                        self.pmutation,
                    )?;
                }
            }
            genetic.optimize()?;
            genetic.initialize(capacity, self.session, Some(shuttle_rate))?;

            if step_cnt < n_train {
                step_cnt = 0;
                breaking_cnt = 0;
                previous_p55 = 0.0;
                previous_diff_p55 = 0.0;
            } else {
                break;
            }
        }

        self.session += n_train as i64;
        Ok(())
    }
}

pub async fn run() -> std::io::Result<()> {
    let mut sim = Simulator::new();
    sim.with_money(1_000_000.0);
    sim.with_stock(0.0);
    sim.with_minimum_stock_buy(1000);
    sim.with_stock_holding_period(10);
    sim.with_sampling(
        "lighttrading.pp.ua",
        "stock",
        "MWG",
        "1D",
        1604607984,
        1755127894,
    )
    .await
    .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;

    for _ in 0..10 {
        sim.with_genetic(1000, 100, 100, 4, 50, 0.1, None)
            .await
            .map_err(|error| Error::new(ErrorKind::InvalidInput, format!("{}", error)))?;
    }
    Ok(())
}
