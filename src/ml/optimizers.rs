//! # Advanced Optimization Algorithms
//!
//! This module provides state-of-the-art optimization algorithms for ML training,
//! including Adam, SGD with momentum, and adaptive learning rate scheduling.

use serde::{Serialize, Deserialize};

/// Common trait for all optimizers
pub trait Optimizer: Send + Sync {
    /// Perform one optimization step
    fn step(&mut self, weights: &mut [f64], gradients: &[f64]);

    /// Get current learning rate
    fn get_learning_rate(&self) -> f64;

    /// Reset optimizer state
    fn reset(&mut self);

    /// Set learning rate
    fn set_learning_rate(&mut self, lr: f64);
}

// ============================================================================
// SGD with Momentum
// ============================================================================

/// Stochastic Gradient Descent with Momentum
///
/// Accelerates SGD by accumulating velocity in directions of consistent gradient.
/// Typical momentum value: 0.9
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SGDMomentum {
    /// Learning rate
    learning_rate: f64,
    /// Momentum coefficient (0.0 to 1.0)
    momentum: f64,
    /// Velocity accumulator
    velocity: Vec<f64>,
}

impl SGDMomentum {
    /// Create new SGD with momentum optimizer
    pub fn new(learning_rate: f64, momentum: f64) -> Self {
        Self {
            learning_rate,
            momentum,
            velocity: Vec::new(),
        }
    }

    /// Initialize velocity vector
    fn ensure_velocity(&mut self, size: usize) {
        if self.velocity.len() != size {
            self.velocity = vec![0.0; size];
        }
    }
}

impl Optimizer for SGDMomentum {
    fn step(&mut self, weights: &mut [f64], gradients: &[f64]) {
        self.ensure_velocity(weights.len());

        for i in 0..weights.len() {
            // Update velocity: v = momentum * v - lr * gradient
            self.velocity[i] = self.momentum * self.velocity[i] - self.learning_rate * gradients[i];

            // Update weights: w = w + v
            weights[i] += self.velocity[i];
        }
    }

    fn get_learning_rate(&self) -> f64 {
        self.learning_rate
    }

    fn reset(&mut self) {
        self.velocity.iter_mut().for_each(|v| *v = 0.0);
    }

    fn set_learning_rate(&mut self, lr: f64) {
        self.learning_rate = lr;
    }
}

// ============================================================================
// Adam Optimizer
// ============================================================================

/// Adam (Adaptive Moment Estimation) Optimizer
///
/// Combines ideas from RMSProp and momentum. Maintains per-parameter adaptive
/// learning rates. Generally the best default optimizer for deep learning.
///
/// Reference: Kingma & Ba, 2014 - "Adam: A Method for Stochastic Optimization"
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdamOptimizer {
    /// Learning rate (alpha)
    learning_rate: f64,
    /// Exponential decay rate for first moment (typically 0.9)
    beta1: f64,
    /// Exponential decay rate for second moment (typically 0.999)
    beta2: f64,
    /// Small constant for numerical stability
    epsilon: f64,
    /// First moment vector (mean of gradients)
    m: Vec<f64>,
    /// Second moment vector (uncentered variance of gradients)
    v: Vec<f64>,
    /// Timestep counter
    t: usize,
}

impl AdamOptimizer {
    /// Create new Adam optimizer with default hyperparameters
    pub fn new(learning_rate: f64) -> Self {
        Self::with_params(learning_rate, 0.9, 0.999, 1e-8)
    }

    /// Create Adam optimizer with custom hyperparameters
    pub fn with_params(learning_rate: f64, beta1: f64, beta2: f64, epsilon: f64) -> Self {
        Self {
            learning_rate,
            beta1,
            beta2,
            epsilon,
            m: Vec::new(),
            v: Vec::new(),
            t: 0,
        }
    }

    /// Initialize moment vectors
    fn ensure_moments(&mut self, size: usize) {
        if self.m.len() != size {
            self.m = vec![0.0; size];
            self.v = vec![0.0; size];
        }
    }
}

impl Optimizer for AdamOptimizer {
    fn step(&mut self, weights: &mut [f64], gradients: &[f64]) {
        self.ensure_moments(weights.len());
        self.t += 1;

        let t_f64 = self.t as f64;

        // Bias correction factors
        let bias_correction1 = 1.0 - self.beta1.powf(t_f64);
        let bias_correction2 = 1.0 - self.beta2.powf(t_f64);

        for i in 0..weights.len() {
            // Update biased first moment estimate
            self.m[i] = self.beta1 * self.m[i] + (1.0 - self.beta1) * gradients[i];

            // Update biased second raw moment estimate
            self.v[i] = self.beta2 * self.v[i] + (1.0 - self.beta2) * gradients[i] * gradients[i];

            // Compute bias-corrected first moment estimate
            let m_hat = self.m[i] / bias_correction1;

            // Compute bias-corrected second raw moment estimate
            let v_hat = self.v[i] / bias_correction2;

            // Update weights
            weights[i] -= self.learning_rate * m_hat / (v_hat.sqrt() + self.epsilon);
        }
    }

    fn get_learning_rate(&self) -> f64 {
        self.learning_rate
    }

    fn reset(&mut self) {
        self.m.iter_mut().for_each(|m| *m = 0.0);
        self.v.iter_mut().for_each(|v| *v = 0.0);
        self.t = 0;
    }

    fn set_learning_rate(&mut self, lr: f64) {
        self.learning_rate = lr;
    }
}

// ============================================================================
// Learning Rate Schedulers
// ============================================================================

/// Learning rate scheduling strategy
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum LRSchedule {
    /// Constant learning rate
    Constant,
    /// Exponential decay: lr = lr0 * gamma^epoch
    ExponentialDecay { gamma: f64 },
    /// Step decay: lr = lr0 * gamma^(epoch // step_size)
    StepDecay { gamma: f64, step_size: usize },
    /// Cosine annealing: lr = lr_min + 0.5 * (lr_max - lr_min) * (1 + cos(pi * epoch / T_max))
    CosineAnnealing { lr_min: f64, t_max: usize },
    /// Linear warmup then exponential decay
    WarmupExponential { warmup_steps: usize, gamma: f64 },
}

/// Learning rate scheduler
pub struct LRScheduler {
    /// Initial learning rate
    initial_lr: f64,
    /// Current learning rate
    current_lr: f64,
    /// Scheduling strategy
    schedule: LRSchedule,
    /// Current epoch/step
    current_step: usize,
}

impl LRScheduler {
    /// Create new scheduler
    pub fn new(initial_lr: f64, schedule: LRSchedule) -> Self {
        Self {
            initial_lr,
            current_lr: initial_lr,
            schedule,
            current_step: 0,
        }
    }

    /// Update learning rate for next epoch/step
    pub fn step(&mut self) -> f64 {
        self.current_step += 1;

        self.current_lr = match self.schedule {
            LRSchedule::Constant => self.initial_lr,

            LRSchedule::ExponentialDecay { gamma } => {
                self.initial_lr * gamma.powf(self.current_step as f64)
            }

            LRSchedule::StepDecay { gamma, step_size } => {
                let decay_count = self.current_step / step_size;
                self.initial_lr * gamma.powf(decay_count as f64)
            }

            LRSchedule::CosineAnnealing { lr_min, t_max } => {
                let progress = (self.current_step % t_max) as f64 / t_max as f64;
                let cosine_term = 0.5 * (1.0 + (std::f64::consts::PI * progress).cos());
                lr_min + (self.initial_lr - lr_min) * cosine_term
            }

            LRSchedule::WarmupExponential { warmup_steps, gamma } => {
                if self.current_step <= warmup_steps {
                    // Linear warmup
                    self.initial_lr * (self.current_step as f64 / warmup_steps as f64)
                } else {
                    // Exponential decay after warmup
                    let decay_steps = self.current_step - warmup_steps;
                    self.initial_lr * gamma.powf(decay_steps as f64)
                }
            }
        };

        self.current_lr
    }

    /// Get current learning rate
    pub fn get_lr(&self) -> f64 {
        self.current_lr
    }

    /// Reset scheduler
    pub fn reset(&mut self) {
        self.current_step = 0;
        self.current_lr = self.initial_lr;
    }
}

// ============================================================================
// Optimizer Factory
// ============================================================================

/// Optimizer type for easy construction
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OptimizerType {
    SGD,
    SGDMomentum,
    Adam,
}

/// Create optimizer from type and learning rate
pub fn create_optimizer(optimizer_type: OptimizerType, learning_rate: f64) -> Box<dyn Optimizer> {
    match optimizer_type {
        OptimizerType::SGD => {
            Box::new(SGDMomentum::new(learning_rate, 0.0)) // momentum = 0 is vanilla SGD
        }
        OptimizerType::SGDMomentum => {
            Box::new(SGDMomentum::new(learning_rate, 0.9))
        }
        OptimizerType::Adam => {
            Box::new(AdamOptimizer::new(learning_rate))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sgd_momentum() {
        let mut optimizer = SGDMomentum::new(0.1, 0.9);
        let mut weights = vec![1.0, 2.0, 3.0];
        let gradients = vec![0.1, 0.2, 0.3];

        optimizer.step(&mut weights, &gradients);

        // Weights should have moved in negative gradient direction
        assert!(weights[0] < 1.0);
        assert!(weights[1] < 2.0);
        assert!(weights[2] < 3.0);
    }

    #[test]
    fn test_adam_optimizer() {
        let mut optimizer = AdamOptimizer::new(0.001);
        let mut weights = vec![1.0, 2.0];
        let gradients = vec![0.1, 0.2];

        optimizer.step(&mut weights, &gradients);

        // Adam should produce smaller steps initially
        assert!((weights[0] - 1.0).abs() < 0.01);
        assert!((weights[1] - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_exponential_decay_schedule() {
        let mut scheduler = LRScheduler::new(
            1.0,
            LRSchedule::ExponentialDecay { gamma: 0.9 },
        );

        let lr1 = scheduler.step();
        assert!((lr1 - 0.9).abs() < 1e-10);

        let lr2 = scheduler.step();
        assert!((lr2 - 0.81).abs() < 1e-10);
    }

    #[test]
    fn test_warmup_schedule() {
        let mut scheduler = LRScheduler::new(
            1.0,
            LRSchedule::WarmupExponential {
                warmup_steps: 10,
                gamma: 0.99,
            },
        );

        // First step should be 0.1 (1.0 * 1/10)
        let lr1 = scheduler.step();
        assert!((lr1 - 0.1).abs() < 1e-10);

        // After warmup, should start decaying
        for _ in 1..15 {
            scheduler.step();
        }

        let lr_after = scheduler.get_lr();
        assert!(lr_after < 1.0);
    }
}


